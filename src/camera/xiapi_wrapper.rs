use xiapi;
use xiapi_sys::*;
use crate::utils::config::CameraConfig;
use super::frame::Frame;

pub struct XiCamera {
    device: xiapi::Camera,
    buffer: Option<xiapi::AcquisitionBuffer>,
}

impl XiCamera {
    pub fn new(config: &CameraConfig) -> Result<Self, i32> {
        let mut device = xiapi::open_device(Some(config.device_id))?;
        Self::configure_camera(&mut device, config)?;

        Ok(XiCamera {
            device,
            buffer: None,
        })
    }

    fn configure_camera(device: &mut xiapi::Camera, config: &CameraConfig) -> Result<(), i32> {
        device.set_exposure(config.exposure)?;
        device.set_acq_timing_mode(xiapi_sys::XI_ACQ_TIMING_MODE::XI_ACQ_TIMING_MODE_FRAME_RATE)?;
        device.set_framerate(config.framerate)?;
        device.set_image_data_format(xiapi::XI_IMG_FORMAT::XI_MONO8)?;
        // Add more configuration options as needed
        Ok(())
    }

    pub fn start_acquisition(&mut self) -> Result<(), i32> {
        self.buffer = Some(self.device.start_acquisition()?);
        Ok(())
    }

    pub fn stop_acquisition(&mut self) -> Result<(), i32> {
        if let Some(buffer) = self.buffer.take() {
            buffer.stop_acquisition()?;
        }
        Ok(())
    }

    pub fn capture_frame(&mut self) -> Result<Frame, anyhow::Error> {
        let buffer = self.buffer.as_ref().ok_or(anyhow::anyhow!("Acquisition buffer not initialized"))?;
        let image = buffer.next_image::<u8>(None);
        Frame::from_xi_image(&image.unwrap())
    }
}

impl Drop for XiCamera {
    fn drop(&mut self) {
        if self.buffer.is_some() {
            let _ = self.stop_acquisition();
        }
    }
}