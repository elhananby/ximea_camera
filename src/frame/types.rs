use image::{ImageBuffer, Luma};
use std::sync::Arc;

#[derive(Clone)]
pub struct Frame {
    pub data: ImageBuffer<Luma<u8>, Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub nframe: u32,
    pub acq_nframe: u32,
    pub timestamp_raw: u64,
    pub exposure_time: u32,
}

impl Frame {
    pub fn new(
        data: ImageBuffer<Luma<u8>, Vec<u8>>,
        width: u32,
        height: u32,
        nframe: u32,
        acq_nframe: u32,
        timestamp_raw: u64,
        exposure_time: u32,
    ) -> Self {
        Self {
            data,
            width,
            height,
            nframe,
            acq_nframe,
            timestamp_raw,
            exposure_time,
        }
    }
}

pub type ArcFrame = Arc<Frame>;