use anyhow::{Result, Context};
use image::{ImageBuffer, Luma, DynamicImage};
use xiapi;
use xiapi_sys::XI_GPI_SELECTOR;

#[derive(Clone)]
pub struct Frame {
    pub data: ImageBuffer<Luma<u8>, Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub nframe: u32,
    pub timestamp: u64,
    pub exposure_time: u32,
}

impl Frame {
    pub fn from_xi_image(xi_image: &xiapi::Image<u8>) -> Result<Self> {
        let width = xi_image.width();
        let height = xi_image.height();
    
        // Extract raw data from xi_image
        let raw_data = xi_image.data(); // Remove the generic argument from the data method call
    
        // Create an ImageBuffer from raw data
        let data = ImageBuffer::<Luma<u8>, Vec<u8>>::from_raw(width, height, raw_data.to_vec())
            .ok_or(anyhow::Error::msg("Failed to create ImageBuffer from raw data"))?;
    
        Ok(Frame {
            data,
            width,
            height,
            nframe: xi_image.nframe(),
            timestamp: xi_image.timestamp_raw(),
            exposure_time: xi_image.exposure_time_us(),
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.data.as_raw()
    }
}