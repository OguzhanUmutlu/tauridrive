use crate::error::{CdpError, CdpResult};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpEvent {
    pub method: String,
    pub params: Value,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

#[derive(Serialize)]
struct CdpRequest {
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Value::is_null")]
    params: Value,
    #[serde(rename = "sessionId", skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
}

#[derive(Deserialize)]
struct CdpResponse {
    id: Option<u64>,
    result: Option<Value>,
    error: Option<CdpResponseError>,
    method: Option<String>,
    params: Option<Value>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CdpResponseError {
    code: i64,
    message: String,
    #[serde(default)]
    #[allow(dead_code)]
    data: Option<Value>,
}

pub struct CdpClient {
    id_counter: AtomicU64,
    cmd_tx: mpsc::Sender<Message>,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<CdpResult<Value>>>>>,
    event_tx: broadcast::Sender<CdpEvent>,
}

impl CdpClient {
    pub async fn connect(ws_url: &str) -> CdpResult<Arc<Self>> {
        let (ws_stream, _) = connect_async(ws_url).await?;
        let (mut ws_sink, mut ws_stream) = ws_stream.split();

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Message>(128);
        let (event_tx, _) = broadcast::channel::<CdpEvent>(1024);

        let pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<CdpResult<Value>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending_requests.clone();
        let event_tx_clone = event_tx.clone();

        // Background write loop
        tokio::spawn(async move {
            while let Some(msg) = cmd_rx.recv().await {
                if ws_sink.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Background read loop
        tokio::spawn(async move {
            while let Some(msg_result) = ws_stream.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        if let Ok(resp) = serde_json::from_str::<CdpResponse>(&text) {
                            if let Some(id) = resp.id {
                                let mut map = pending_clone.lock().await;
                                if let Some(sender) = map.remove(&id) {
                                    if let Some(err) = resp.error {
                                        let _ = sender.send(Err(CdpError::Protocol {
                                            code: err.code,
                                            message: err.message,
                                        }));
                                    } else {
                                        let _ = sender.send(Ok(resp.result.unwrap_or(Value::Null)));
                                    }
                                }
                            } else if let Some(method) = resp.method {
                                let event = CdpEvent {
                                    method,
                                    params: resp.params.unwrap_or(Value::Null),
                                    session_id: resp.session_id,
                                };
                                let _ = event_tx_clone.send(event);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
        });

        Ok(Arc::new(Self {
            id_counter: AtomicU64::new(1),
            cmd_tx,
            pending_requests,
            event_tx,
        }))
    }

    pub async fn send_command(&self, method: &str, params: Value) -> CdpResult<Value> {
        self.send_command_with_session(method, params, None).await
    }

    pub async fn send_command_with_session(
        &self,
        method: &str,
        params: Value,
        session_id: Option<String>,
    ) -> CdpResult<Value> {
        let id = self.id_counter.fetch_add(1, Ordering::SeqCst);
        let req = CdpRequest {
            id,
            method: method.to_string(),
            params,
            session_id,
        };
        let text = serde_json::to_string(&req)?;

        let (tx, rx) = oneshot::channel();
        {
            let mut map = self.pending_requests.lock().await;
            map.insert(id, tx);
        }

        self.cmd_tx
            .send(Message::Text(text.into()))
            .await
            .map_err(|_| CdpError::WebSocket(tokio_tungstenite::tungstenite::Error::ConnectionClosed))?;

        rx.await.map_err(|_| {
            CdpError::Timeout("Response channel closed before receiving CDP result".to_string())
        })?
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CdpEvent> {
        self.event_tx.subscribe()
    }
}
