use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Camera error: {0}")]
    CameraError(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Communication error: {0}")]
    CommunicationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unexpected error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Other(err.to_string())
    }
}