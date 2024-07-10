use anyhow::{Context, Result};
use config::{Config as ConfigCrate, File, FileFormat};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub camera: CameraConfig,
    pub processing: ProcessingConfig,
    pub zmq: ZmqConfig,
    pub log: LogConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CameraConfig {
    pub device_id: u32,
    pub exposure: f32,
    pub framerate: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessingConfig {
    pub buffer_size: usize,
    pub frames_after_trigger: usize,
    pub video_config: VideoConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VideoConfig {
    pub framerate: f32,
    pub codec: String,
    pub crf: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ZmqConfig {
    pub sub_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogConfig {
    pub level: String,
    pub file: Option<String>,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let config = ConfigCrate::builder()
            .add_source(File::new(path, FileFormat::Toml))
            .build()
            .context("Failed to build configuration")?;

        config
            .try_deserialize()
            .context("Failed to deserialize configuration")
    }
}
