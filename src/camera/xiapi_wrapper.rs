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

        // Resolution
        let roi = xiapi::Roi {
            offset_x: config.offset_x,
            offset_y: config.offset_y,
            width: config.width,
            height: config.height,
        };

        device.set_roi(&roi).map_err(|e| anyhow!("Failed to set ROI: {}", e))?;

        // Timing mode and framerate
        device
            .set_acq_timing_mode(xiapi::XI_ACQ_TIMING_MODE::XI_ACQ_TIMING_MODE_FRAME_RATE_LIMIT)
            .map_err(|e| anyhow!("Failed to set acquisition timing mode: {}", e))?;

        device
            .set_framerate(config.framerate)
            .map_err(|e| anyhow!("Failed to set frame rate: {}", e))?;

        // Data formate
        device
            .set_image_data_format(xiapi::XI_IMG_FORMAT::XI_MONO8)
            .map_err(|e| anyhow!("Failed to set image data format to XI_MONO8: {}", e))?;

        // Bandwidth
        let maximum_bandwidth = device
            .limit_bandwidth_maximum()
            .map_err(|e| anyhow!("Failed to get maximum bandwidth limit: {}", e))?;

        device
            .set_limit_bandwidth(maximum_bandwidth)
            .map_err(|e| anyhow!("Failed to set bandwidth limit to maximum: {}", e))?;

        // Buffer size
        let buffer_size = device
            .acq_buffer_size()
            .map_err(|e| anyhow!("Failed to get acquisition buffer size: {}", e))?;

        device.set_acq_buffer_size(buffer_size * 4).map_err(|e| {
            anyhow!(
                "Failed to set acquisition buffer size to {} bytes: {}",
                buffer_size * 4,
                e
            )
        })?;

        // Queue size
        let buffer_queue_maximum = device
            .buffers_queue_size_maximum()
            .map_err(|e| anyhow!("Failed to get maximum buffer queue size: {}", e))?;

        device
            .set_buffers_queue_size(buffer_queue_maximum)
            .map_err(|e| anyhow!("Failed to set buffer queue size to maximum: {}", e))?;

        // unsafe AEAG
        unsafe {
            xiapi::xiSetParamInt(
                **device,
                xiapi::XI_PRM_AEAG.as_ptr() as *const i8,
                xiapi::XI_SWITCH::XI_ON.try_into().unwrap(),
            );
            xiapi::xiSetParamFloat(
                **device,
                xiapi::XI_PRM_EXP_PRIORITY.as_ptr() as *const i8,
                1.0,
            );
            xiapi::xiSetParamInt(
                **device,
                xiapi::XI_PRM_AE_MAX_LIMIT.as_ptr() as *const i8,
                2000,
            );
            xiapi::xiSetParamFloat(
                **device,
                xiapi::XI_PRM_AEAG_LEVEL.as_ptr() as *const i8,
                75.0,
            );
        }

        // recent frame
        device
            .recent_frame()
            .map_err(|e| anyhow!("Failed to get recent frame: {}", e))?;

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
