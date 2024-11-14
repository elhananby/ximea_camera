use anyhow::{Context, Result};
use log::{info, debug, error};
use xiapi;

use crate::camera::CameraConfig;

pub struct XimeaCamera {
    device: xiapi::Camera,
}

impl XimeaCamera {
    pub fn new(config: &CameraConfig) -> result<Self> {
        info!("Initializing XIMEA camera");
        let mut device = xiapi::open_device(Some(config.serial)).context("Failed to open XIMEA camera");
        let camera = Self { device };
        camera.configure(config)?;
        Ok(camera)
    }

    fn configure(&self, config: &CameraConfig) -> Result<()> {
        debug!("Configuring camera parameters");

        self.set_resolution(config.width, config.height, config.offset_x, config.offset_y)?;
        self.set_framerate(config.fps)?;
        self.set_exposure(config.exposure)?;
        self.set_image_format()?;
        self.optimize_buffers()?;

        self.device.reset_frame()?;

        info!("Camera configuration complete");
        Ok(())
    }

    fn set_resolution(&self, width: u32, height: u32, offset_x: u32, offset_y: u32) -> Result<()> {
        let roi = xiapi::Roi{
            offset_x,
            offset_y,
            width,
            height,
        };
        self.device.set_roi(&roi).context("Failed to set camera resolution")?;
        debug!("Resolution set to {}x{} with offset ({}, {})", width, height, offset_x, offset_y);
        Ok(())
    }

    fn set_framerate(&self, fps: f32) -> Result<()> {
        self.device.set_acq_timing_mode(xiapi::XI_ACQ_TIMING_MODE::XI_ACQ_TIMING_MODE_FRAME_RATE_LIMIT).context("Failed to set framerate mode")?;
        self.device.set_framerate(fps).context("Failed to set framerate")?;
        Ok(())
    }

    fn set_exposure(&self, exposure: f32) -> Result<()> {
        self.device.set_exposure(exposure).context("Failed to set exposure")?;
        debug!("Exposure set to {} µs", exposure);
        Ok(())
    }

    fn set_image_format(&self) -> Result<()> {
        self.device.set_image_data_format(xiapi::XI_IMG_FORMAT::XI_MONO8)
            .context("Failed to set image format to MONO8")?;
        debug!("Image format set to MONO8");
        Ok(())
    }

    fn optimize_buffers(&self) -> Result<()> {
        let max_bandwidth = self.device.limit_bandwidth_maximum()?;
        self.device.set_limit_bandwidth(max_bandwidth)
            .context("Failed to set bandwidth limit")?;

        let buffer_size = self.device.acq_buffer_size()?;
        self.device.set_acq_buffer_size(buffer_size * 4)
            .context("Failed to set acquisition buffer size")?;

        let max_queue_size = self.device.buffers_queue_size_maximum()?;
        self.device.set_buffers_queue_size(max_queue_size)
            .context("Failed to set buffers queue size")?;

        debug!("Buffers parameters optimized");
        Ok(())
    }

    fn start_acquisition(&self) -> Result<()> {
        self.device.start_acquisition().context("Failed to start acquisition")
    }

    fn stop_acquisition(&self) -> Result<()> {
        self.device.stop_acquisition().context("Failed to stop acquisition")
    }
}

impl Drop for XimeaCamera {
    fn drop(&mut self) {
        if let Err(e) = self.stop_acquisition() {
            error!("Failed to stop acquisition on camera drop {:?}", e);
        }
        info!("Closing XIMEA camera");
    }
}