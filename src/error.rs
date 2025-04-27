use thiserror::Error;

#[derive(Error, Debug)]
pub enum RStreamerError {
    #[error("Camera error: {0}")]
    CameraError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("axum error: {0}")]
    ServerError(String),
    #[error("Broadcast error: {0}")]
    BroadcastError(String),
}

pub type Result<T> = std::result::Result<T, RStreamerError>;