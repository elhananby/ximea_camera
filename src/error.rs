use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Camera error: {0}")]
    Camera(#[from] CameraError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Frame processing error: {0}")]
    FrameProcessing(String),

    #[error("Video saving error: {0}")]
    VideoSaving(String),

    #[error("ZMQ communication error: {0}")]
    ZmqCommunication(#[from] zmq::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

#[derive(Error, Debug)]
pub enum CameraError {
    #[error("Failed to initialize camera: {0}")]
    Initialization(String),

    #[error("Failed to configure camera: {0}")]
    Configuration(String),

    #[error("Failed to start acquisition: {0}")]
    StartAcquisition(String),

    #[error("Failed to stop acquisition: {0}")]
    StopAcquisition(String),

    #[error("Failed to capture frame: {0}")]
    FrameCapture(String),

    #[error("Camera not found: {0}")]
    NotFound(String),
}

impl From<xiapi::Error> for AppError {
    fn from(error: xiapi::Error) -> Self {
        AppError::Camera(CameraError::Initialization(error.to_string()))
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

// Helper functions for creating errors
impl AppError {
    pub fn config(msg: impl Into<String>) -> Self {
        AppError::Config(msg.into())
    }

    pub fn frame_processing(msg: impl Into<String>) -> Self {
        AppError::FrameProcessing(msg.into())
    }

    pub fn video_saving(msg: impl Into<String>) -> Self {
        AppError::VideoSaving(msg.into())
    }

    pub fn unknown(msg: impl Into<String>) -> Self {
        AppError::Unknown(msg.into())
    }
}

impl CameraError {
    pub fn initialization(msg: impl Into<String>) -> Self {
        CameraError::Initialization(msg.into())
    }

    pub fn configuration(msg: impl Into<String>) -> Self {
        CameraError::Configuration(msg.into())
    }

    pub fn start_acquisition(msg: impl Into<String>) -> Self {
        CameraError::StartAcquisition(msg.into())
    }

    pub fn stop_acquisition(msg: impl Into<String>) -> Self {
        CameraError::StopAcquisition(msg.into())
    }

    pub fn frame_capture(msg: impl Into<String>) -> Self {
        CameraError::FrameCapture(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        CameraError::NotFound(msg.into())
    }
}