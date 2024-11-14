use serde::{Deserialize, Serialize};

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct CameraConfig {
    pub serial: u32,
    pub fps: f32,
    pub exposure: f32,
    pub width: u32,
    pub height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            serial: 0,
            fps: 500.0,
            exposure: 2000.0,
            width: 2016,
            height: 2016,
            offset_x: 1056,
            offset_y: 170,
        }
    }
}

impl CameraConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_serial(mut self, serial: u32) -> Self {
        self.serial = serial;
        self
    }

    pub fn with_fps(mut self, fps: f32) -> Self {
        self.fps = fps;
        self
    }

    pub fn with_exposure(mut self, exposure: f32) -> Self {
        self.exposure = exposure;
        self
    }

    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_offset(mut self, offset_x: u32, offset_y: u32) -> Self {
        self.offset_x = offset_x;
        self.offset_y = offset_y;
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.fps <= 0.0 {
            return Err("FPS must be greater than 0".to_string());
        }
        if self.exposure <= 0.0 {
            return Err("Exposure must be greater than 0".to_string());
        }
        if self.width == 0 || self.height == 0 {
            return Err("Resolution must be greater than 0".to_string());
        }
        Ok(())
    }
}