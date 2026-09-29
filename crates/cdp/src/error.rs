use thiserror::Error;

pub type CdpResult<T> = Result<T, CdpError>;

#[derive(Error, Debug)]
pub enum CdpError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Protocol error ({code}): {message}")]
    Protocol { code: i64, message: String },
    #[error("Target error: {0}")]
    Target(String),
    #[error("Timeout error: {0}")]
    Timeout(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
