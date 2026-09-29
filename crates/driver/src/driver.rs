use crate::capture::{ScreenshotFormat, ScreenshotOptions};
use crate::element::ElementRect;
use crate::error::{DriverError, DriverResult};
use crate::input::MouseButton;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauridrive_cdp::{CdpClient, Discovery};

pub struct Driver {
    client: Arc<CdpClient>,
    mouse_x: f64,
    mouse_y: f64,
}

impl Driver {
    pub async fn connect(ws_url: &str) -> DriverResult<Self> {
        let client = CdpClient::connect(ws_url).await?;
        let driver = Self {
            client,
            mouse_x: 0.0,
            mouse_y: 0.0,
        };
        // Enable necessary CDP domains
        driver.client.send_command("Page.enable", json!({})).await?;
        driver.client.send_command("Runtime.enable", json!({})).await?;
        driver.client.send_command("DOM.enable", json!({})).await?;
        Ok(driver)
    }

    pub async fn attach_to_target(host: &str, port: u16, timeout: Duration) -> DriverResult<Self> {
        let discovery = Discovery::new(host, port);
        let target = discovery.wait_for_target(timeout).await?;
        let ws_url = target.websocket_debugger_url.ok_or_else(|| {
            DriverError::Cdp(tauridrive_cdp::CdpError::Target(
                "Target has no webSocketDebuggerUrl".to_string(),
            ))
        })?;
        Self::connect(&ws_url).await
    }

    pub fn client(&self) -> Arc<CdpClient> {
        self.client.clone()
    }

    pub async fn eval(&self, expression: &str) -> DriverResult<serde_json::Value> {
        let params = json!({
            "expression": expression,
            "awaitPromise": true,
            "returnByValue": true,
            "includeCommandLineAPI": true,
        });
        let res = self.client.send_command("Runtime.evaluate", params).await?;
        
        if let Some(exception_details) = res.get("exceptionDetails") {
            let text = exception_details
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("Unknown runtime exception");
            let desc = exception_details
                .get("exception")
                .and_then(|e| e.get("description"))
                .and_then(|d| d.as_str())
                .unwrap_or(text);
            return Err(DriverError::ScriptEvaluation(desc.to_string()));
        }

        let val = res
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        Ok(val)
    }

    pub async fn eval_file(&self, path: &Path) -> DriverResult<serde_json::Value> {
        let script = std::fs::read_to_string(path)?;
        self.eval(&script).await
    }

    pub async fn get_element_rect(&self, selector: &str) -> DriverResult<ElementRect> {
        let script = format!(
            r#"(() => {{
                const el = document.querySelector({});
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return {{ x: r.left, y: r.top, width: r.width, height: r.height }};
            }})()"#,
            serde_json::to_string(selector).unwrap()
        );
        let res = self.eval(&script).await?;
        if res.is_null() {
            return Err(DriverError::ElementNotFound(selector.to_string()));
        }
        let rect: ElementRect = serde_json::from_value(res)
            .map_err(|e| DriverError::ScriptEvaluation(format!("Failed to parse rect: {}", e)))?;
        Ok(rect)
    }

    pub async fn wait_for_selector(&self, selector: &str, timeout: Duration) -> DriverResult<ElementRect> {
        let start = std::time::Instant::now();
        let poll = Duration::from_millis(100);
        while start.elapsed() < timeout {
            if let Ok(rect) = self.get_element_rect(selector).await {
                return Ok(rect);
            }
            tokio::time::sleep(poll).await;
        }
        Err(DriverError::ElementNotFound(format!(
            "Timeout waiting for selector: {}",
            selector
        )))
    }

    pub async fn move_to(&mut self, x: f64, y: f64) -> DriverResult<()> {
        self.mouse_x = x;
        self.mouse_y = y;
        self.client
            .send_command(
                "Input.dispatchMouseEvent",
                json!({
                    "type": "mouseMoved",
                    "x": x,
                    "y": y,
                }),
            )
            .await?;
        Ok(())
    }

    pub async fn hover(&mut self, selector: &str) -> DriverResult<()> {
        let rect = self.get_element_rect(selector).await?;
        let (cx, cy) = rect.center();
        self.move_to(cx, cy).await?;
        Ok(())
    }

    pub async fn click(&mut self, selector: &str) -> DriverResult<()> {
        let rect = self.get_element_rect(selector).await?;
        let (cx, cy) = rect.center();
        self.click_at(cx, cy, MouseButton::Left, 1).await
    }

    pub async fn click_at(&mut self, x: f64, y: f64, button: MouseButton, count: i32) -> DriverResult<()> {
        self.move_to(x, y).await?;
        let btn_str = button.as_str();

        self.client
            .send_command(
                "Input.dispatchMouseEvent",
                json!({
                    "type": "mousePressed",
                    "x": x,
                    "y": y,
                    "button": btn_str,
                    "clickCount": count,
                }),
            )
            .await?;

        tokio::time::sleep(Duration::from_millis(30)).await;

        self.client
            .send_command(
                "Input.dispatchMouseEvent",
                json!({
                    "type": "mouseReleased",
                    "x": x,
                    "y": y,
                    "button": btn_str,
                    "clickCount": count,
                }),
            )
            .await?;

        Ok(())
    }

    pub async fn type_text(&self, text: &str) -> DriverResult<()> {
        for ch in text.chars() {
            self.client
                .send_command(
                    "Input.dispatchKeyEvent",
                    json!({
                        "type": "keyDown",
                        "text": ch.to_string(),
                        "unmodifiedText": ch.to_string(),
                    }),
                )
                .await?;
            self.client
                .send_command(
                    "Input.dispatchKeyEvent",
                    json!({
                        "type": "keyUp",
                    }),
                )
                .await?;
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
        Ok(())
    }

    pub async fn press_key(&self, key: &str) -> DriverResult<()> {
        self.client
            .send_command(
                "Input.dispatchKeyEvent",
                json!({
                    "type": "rawKeyDown",
                    "key": key,
                }),
            )
            .await?;
        tokio::time::sleep(Duration::from_millis(20)).await;
        self.client
            .send_command(
                "Input.dispatchKeyEvent",
                json!({
                    "type": "keyUp",
                    "key": key,
                }),
            )
            .await?;
        Ok(())
    }

    pub async fn screenshot(&self, options: ScreenshotOptions) -> DriverResult<Vec<u8>> {
        let mut params = json!({
            "format": options.format.as_str(),
            "fromSurface": options.from_surface,
            "captureBeyondViewport": options.capture_beyond_viewport,
        });

        if let Some(quality) = options.quality {
            params["quality"] = json!(quality);
        }

        if let Some(clip) = options.clip {
            params["clip"] = json!({
                "x": clip.x,
                "y": clip.y,
                "width": clip.width,
                "height": clip.height,
                "scale": 1.0,
            });
        }

        let res = self.client.send_command("Page.captureScreenshot", params).await?;
        let b64_data = res
            .get("data")
            .and_then(|d| d.as_str())
            .ok_or_else(|| DriverError::Capture("No screenshot data in CDP response".to_string()))?;

        let bytes = BASE64.decode(b64_data)?;
        Ok(bytes)
    }

    pub async fn screenshot_element(&self, selector: &str) -> DriverResult<Vec<u8>> {
        let rect = self.get_element_rect(selector).await?;
        let options = ScreenshotOptions {
            clip: Some(rect),
            ..Default::default()
        };
        self.screenshot(options).await
    }

    pub async fn start_screencast(&self, format: ScreenshotFormat, quality: Option<u8>) -> DriverResult<()> {
        let mut params = json!({
            "format": format.as_str(),
            "everyNthFrame": 1,
        });
        if let Some(q) = quality {
            params["quality"] = json!(q);
        }
        self.client.send_command("Page.startScreencast", params).await?;
        Ok(())
    }

    pub async fn stop_screencast(&self) -> DriverResult<()> {
        self.client.send_command("Page.stopScreencast", json!({})).await?;
        Ok(())
    }

    pub async fn ack_screencast_frame(&self, session_id: u32) -> DriverResult<()> {
        self.client
            .send_command("Page.screencastFrameAck", json!({ "sessionId": session_id }))
            .await?;
        Ok(())
    }

    pub async fn step_frame(&self, interval_ms: f64, screenshot: bool) -> DriverResult<Option<Vec<u8>>> {
        let mut params = json!({
            "interval": interval_ms,
        });
        if screenshot {
            params["screenshot"] = json!({ "format": "png" });
        }
        let res = self.client.send_command("HeadlessExperimental.beginFrame", params).await?;
        if screenshot {
            if let Some(b64_data) = res.get("screenshotData").and_then(|d| d.as_str()) {
                let bytes = BASE64.decode(b64_data)?;
                return Ok(Some(bytes));
            }
        }
        Ok(None)
    }

    pub async fn set_animation_playback_rate(&self, playback_rate: f64) -> DriverResult<()> {
        self.client.send_command("Animation.enable", json!({})).await?;
        self.client
            .send_command(
                "Animation.setPlaybackRate",
                json!({ "playbackRate": playback_rate }),
            )
            .await?;
        Ok(())
    }
}
