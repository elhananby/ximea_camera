use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TriggerMessage {
    pub obj_id: u32,
    pub frame: u64,
    pub timestamp: f64,
}

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub trigger: TriggerMessage,
    pub path: PathBuf,
    pub frame_count: usize,
}

#[derive(Debug, Clone)]
pub struct FrameMetadata {
    pub frame_number: u64,
    pub timestamp: f64,
    pub exposure_time: u32,
}

#[derive(Debug, Clone)]
pub enum SystemEvent {
    ProcessorStarted,
    ProcessorStopped,
    BufferingComplete,
    RecordingStarted(TriggerMessage),
    VideoSaved(VideoMetadata),
    Error(String),
}
// // If we need to represent different types of errors in a structured way
// #[derive(Debug, Clone)]
// pub enum AppErrorType {
//     CameraError,
//     ProcessingError,
//     CommunicationError,
//     ConfigurationError,
//     IOError,
// }

// // If we need a result type that uses our custom error
// pub type AppResult<T> = std::result::Result<T, crate::utils::error::AppError>;
