//! A library for controlling XIMEA cameras and processing high-speed video.
//!
//! This library provides functionality for:
//! - Configuring and controlling XIMEA cameras
//! - Capturing and processing high-speed video frames
//! - Handling trigger-based recording
//! - Saving videos and metadata

pub mod camera;
pub mod cli;
pub mod config;
pub mod error;
pub mod frame;
pub mod logging;
pub mod messaging;
pub mod video;

pub use camera::XimeaCamera;
pub use config::Config;
pub use error::{AppError, Result};
pub use frame::{Frame, FrameBuffer, FrameProcessor};
pub use messaging::{ZmqSubscriber, MessageType};
pub use video::{VideoSaver, save_video_metadata};

// Rest of the lib.rs content...
/// Version of the library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the library
///
/// This function should be called before using any other functionality
/// from the library. It sets up logging and performs any necessary
/// global initialization.
///
/// # Arguments
///
/// * `debug` - Whether to enable debug logging
/// * `log_file` - Optional path to a log file. If None, logs will only be output to stdout.
///
/// # Returns
///
/// A `Result` indicating success or failure of initialization.
pub fn initialize(debug: bool, log_file: Option<&str>) -> Result<()> {
    logging::setup_logging(debug as u8, log_file)?;
    logging::log_app_start(VERSION);
    Ok(())
}

/// A convenience function to create a new camera instance with the given configuration
///
/// # Arguments
///
/// * `config` - The configuration for the camera
///
/// # Returns
///
/// A `Result` containing the new `XimeaCamera` instance if successful, or an error if not.
pub fn new_camera(config: &config::CameraConfig) -> Result<XimeaCamera> {
    XimeaCamera::new(config).map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty(), "Version should not be empty");
    }

    // Add more tests as needed
}