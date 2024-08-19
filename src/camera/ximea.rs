use super::Camera;
use super::CameraConfig;
use crate::error::Error;
use xiapi::{self};
use xiapi_sys::{XI_IMG_FORMAT, XI_RET};
use std::sync::Arc;

pub struct XimeaCamera {
    device: xiapi::Camera,
    config: CameraConfig,
}

impl XimeaCamera {
    pub fn new(config: CameraConfig) -> Result<Self, Error> {
        let device = xiapi::open_device(Some(config.serial))
            .map_err(|e| Error::CameraError(format!("Failed to open camera: {}", e)))?;
        
        let mut camera = Self { device, config };
        camera.configure()?;

        Ok(camera)
    }

    fn configure(&mut self) -> Result<(), Error> {
        // Set camera parameters based on the config
        self.set_image_data_format(XI_IMG_FORMAT::XI_MONO8)?;
        self.set_acq_timing_mode(xiapi::XI_ACQ_TIMING_MODE::XI_ACQ_TIMING_MODE_FRAME_RATE_LIMIT)?;
        self.set_framerate(self.config.fps)?;
        self.set_exposure(self.config.exposure)?;
        self.set_gain(0.0)?;  // Set default gain

        // Set ROI
        let roi = xiapi::Roi {
            offset_x: self.config.offset_x,
            offset_y: self.config.offset_y,
            width: self.config.width,
            height: self.config.height,
        };
        self.set_roi(&roi)?;

        Ok(())
    }

    pub fn start_acquisition(self) -> Result<xiapi::AcquisitionBuffer, Error> {
        self.device.start_acquisition()
            .map_err(|e| Error::CameraError(format!("Failed to start acquisition: {}", e)))
    }
}

impl Camera for XimeaCamera {
    fn capture_frame(&self) -> Result<Arc<Vec<u8>>, Error> {
        Err(Error::CameraError("Capture frame not implemented for XimeaCamera. Use start_acquisition() instead.".to_string()))
    }

    fn start_acquisition(&mut self) -> Result<(), Error> {
        Err(Error::CameraError("Use XimeaCamera::start_acquisition() which consumes the camera and returns an AcquisitionBuffer".to_string()))
    }

    fn stop_acquisition(&mut self) -> Result<(), Error> {
        Err(Error::CameraError("Stop acquisition not implemented for XimeaCamera. Use AcquisitionBuffer::stop_acquisition() instead.".to_string()))
    }
}

// Implement setters for camera parameters
impl XimeaCamera {
    pub fn set_image_data_format(&mut self, format: XI_IMG_FORMAT) -> Result<(), Error> {
        self.device.set_image_data_format(format)
            .map_err(|e| Error::CameraError(format!("Failed to set image data format: {}", e)))
    }

    pub fn set_acq_timing_mode(&mut self, mode: xiapi::XI_ACQ_TIMING_MODE) -> Result<(), Error> {
        self.device.set_acq_timing_mode(mode)
            .map_err(|e| Error::CameraError(format!("Failed to set acquisition timing mode: {}", e)))
    }

    pub fn set_framerate(&mut self, fps: f32) -> Result<(), Error> {
        self.device.set_framerate(fps)
            .map_err(|e| Error::CameraError(format!("Failed to set framerate: {}", e)))
    }

    pub fn set_exposure(&mut self, exposure: f32) -> Result<(), Error> {
        self.device.set_exposure(exposure)
            .map_err(|e| Error::CameraError(format!("Failed to set exposure: {}", e)))
    }

    pub fn set_gain(&mut self, gain: f32) -> Result<(), Error> {
        self.device.set_gain(gain)
            .map_err(|e| Error::CameraError(format!("Failed to set gain: {}", e)))
    }

    pub fn set_roi(&mut self, roi: &xiapi::Roi) -> Result<(), Error> {
        self.device.set_roi(roi)
            .map_err(|e| Error::CameraError(format!("Failed to set ROI: {}", e)))
            .map(|_| ())
    }
}