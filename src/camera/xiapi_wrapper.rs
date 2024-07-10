use crate::camera::frame::Frame;
use crate::utils::config::CameraConfig;
use anyhow::{anyhow, Result};
use xiapi;

pub struct XiCamera {
    device: Option<xiapi::Camera>,
    buffer: Option<xiapi::AcquisitionBuffer>,
}

impl XiCamera {
    pub fn new(config: &CameraConfig) -> Result<Self> {
        let mut device = xiapi::open_device(Some(config.device_id))
            .map_err(|e| anyhow!("Failed to open XIMEA camera: {}", e))?;

        Self::configure_camera(&mut device, config)?;

        Ok(XiCamera {
            device: Some(device),
            buffer: None,
        })
    }

    fn configure_camera(device: &mut xiapi::Camera, config: &CameraConfig) -> Result<()> {
        device
            .set_exposure(config.exposure)
            .map_err(|e| anyhow!("Failed to set exposure: {}", e))?;
        device
            .set_framerate(config.framerate)
            .map_err(|e| anyhow!("Failed to set frame rate: {}", e))?;
        // device
        //     .set_image_data_format(xiapi::ImgFormat::Mono8)
        //     .map_err(|e| anyhow!("Failed to set image format: {}", e))?;
        // // Add more configuration options as needed
        Ok(())
    }

    pub fn start_acquisition(&mut self) -> Result<()> {
        if let Some(device) = self.device.take() {
            self.buffer = Some(
                device
                    .start_acquisition()
                    .map_err(|e| anyhow!("Failed to start acquisition: {}", e))?,
            );
            Ok(())
        } else {
            Err(anyhow!("Camera device is not available"))
        }
    }

    pub fn stop_acquisition(&mut self) -> Result<()> {
        if let Some(buffer) = self.buffer.take() {
            let device = buffer
                .stop_acquisition()
                .map_err(|e| anyhow!("Failed to stop acquisition: {}", e))?;
            self.device = Some(device);
            Ok(())
        } else {
            Err(anyhow!("Acquisition is not started"))
        }
    }

    pub fn capture_frame(&mut self) -> Result<Frame> {
        let buffer = self
            .buffer
            .as_ref()
            .ok_or_else(|| anyhow!("Acquisition not started"))?;

        let image = buffer
            .next_image::<u8>(None)
            .map_err(|e| anyhow!("Failed to capture frame: {}", e))?;

        Frame::from_xi_image(&image)
    }

    pub fn set_exposure(&mut self, exposure: f32) -> Result<()> {
        if let Some(device) = &mut self.device {
            device
                .set_exposure(exposure)
                .map_err(|e| anyhow!("Failed to set exposure: {}", e))
        } else {
            Err(anyhow!("Camera device is not available"))
        }
    }

    pub fn set_framerate(&mut self, fps: f32) -> Result<()> {
        if let Some(device) = &mut self.device {
            device
                .set_framerate(fps)
                .map_err(|e| anyhow!("Failed to set frame rate: {}", e))
        } else {
            Err(anyhow!("Camera device is not available"))
        }
    }

    // Add methods for other camera settings as needed
}

impl Drop for XiCamera {
    fn drop(&mut self) {
        if self.buffer.is_some() {
            let _ = self.stop_acquisition();
        }
    }
}
