use thiserror::Error;

pub type BenchResult<T> = Result<T, BenchError>;

#[derive(Error, Debug)]
pub enum BenchError {
    #[error("Driver error: {0}")]
    Driver(#[from] tauridrive_driver::DriverError),
    #[error("CDP error: {0}")]
    Cdp(#[from] tauridrive_cdp::CdpError),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Diff error: {0}")]
    Diff(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
