use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Camera error: {0}")]
    CameraError(String),

    #[error("Messaging error: {0}")]
    MessagingError(String),

    #[error("Video processing error: {0}")]
    VideoError(String),

    #[error("FFmpeg error: {0}")]
    FFmpegError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("ZMQ error: {0}")]
    ZmqError(#[from] zmq::Error),

    #[error("Xiapi error: {0}")]
    XiapiError(i32),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl Error {
    pub fn from_xiapi_error(error_code: i32) -> Self {
        Error::XiapiError(error_code)
    }
}