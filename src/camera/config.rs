use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            return Err("Width and height must be greater than 0".to_string());
        }
        Ok(())
    }

    pub fn adjusted_exposure(&self) -> f32 {
        let max_exposure_for_fps = 1_000_000.0 / self.fps;
        if self.exposure > max_exposure_for_fps {
            max_exposure_for_fps - 1.0
        } else {
            self.exposure
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CameraConfig::default();
        assert_eq!(config.serial, 0);
        assert_eq!(config.fps, 500.0);
        assert_eq!(config.exposure, 2000.0);
        assert_eq!(config.width, 2016);
        assert_eq!(config.height, 2016);
        assert_eq!(config.offset_x, 1056);
        assert_eq!(config.offset_y, 170);
    }

    #[test]
    fn test_config_builder() {
        let config = CameraConfig::new()
            .with_serial(1)
            .with_fps(1000.0)
            .with_exposure(1000.0)
            .with_resolution(1920, 1080)
            .with_offset(0, 0);

        assert_eq!(config.serial, 1);
        assert_eq!(config.fps, 1000.0);
        assert_eq!(config.exposure, 1000.0);
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.offset_x, 0);
        assert_eq!(config.offset_y, 0);
    }

    #[test]
    fn test_validate_valid_config() {
        let config = CameraConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_config() {
        let mut config = CameraConfig::default();
        config.fps = 0.0;
        assert!(config.validate().is_err());

        config.fps = 500.0;
        config.exposure = 0.0;
        assert!(config.validate().is_err());

        config.exposure = 2000.0;
        config.width = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_adjusted_exposure() {
        let mut config = CameraConfig::default();
        config.fps = 1000.0;
        config.exposure = 1500.0;
        assert_eq!(config.adjusted_exposure(), 999.0);

        config.exposure = 500.0;
        assert_eq!(config.adjusted_exposure(), 500.0);
    }
}