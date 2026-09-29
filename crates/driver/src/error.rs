use thiserror::Error;

pub type DriverResult<T> = Result<T, DriverError>;

#[derive(Error, Debug)]
pub enum DriverError {
    #[error("CDP error: {0}")]
    Cdp(#[from] tauridrive_cdp::CdpError),
    #[error("Element not found: {0}")]
    ElementNotFound(String),
    #[error("Script evaluation error: {0}")]
    ScriptEvaluation(String),
    #[error("Capture error: {0}")]
    Capture(String),
    #[error("Animation error: {0}")]
    Animation(String),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
