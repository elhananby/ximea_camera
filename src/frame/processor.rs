use super::types::{Frame, ArcFrame};
use anyhow::Result;
use image::{ImageBuffer, Luma};
use log::debug;
use std::sync::Arc;

pub struct FrameProcessor;

impl FrameProcessor {
    pub fn process_frame(frame: &xiapi::Image) -> Result<ArcFrame> {
        debug!("Processing frame {}", frame.nframe());

        let width = frame.width();
        let height = frame.height();
        let image_data: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, frame.as_slice().to_vec())
            .ok_or_else(|| anyhow::anyhow!("Failed to create ImageBuffer"))?;

        let processed_frame = Frame::new(
            image_data,
            width,
            height,
            frame.nframe(),
            frame.acq_nframe(),
            frame.timestamp_raw(),
            frame.exposure_time_us(),
        );

        Ok(Arc::new(processed_frame))
    }

    // Add more processing functions as needed
}