mod ximea;

pub use crate::config::CameraConfig;
pub use ximea::XimeaCamera;

use std::sync::Arc;
use crate::error::Error;

pub trait Camera {
    fn capture_frame(&self) -> Result<Arc<Vec<u8>>, Error>;
    fn start_acquisition(&mut self) -> Result<(), Error>;
    fn stop_acquisition(&mut self) -> Result<(), Error>;
}

pub fn open_camera(config: CameraConfig) -> Result<XimeaCamera, Error> {
    XimeaCamera::new(config)
}