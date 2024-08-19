use crate::error::Error;
use crate::messaging::Message;
use super::{VideoWriter, FfmpegWriter};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use image::{ImageBuffer, Luma};

pub struct FrameHandler {
    buffer: VecDeque<Arc<ImageData>>,
    n_before: usize,
    n_after: usize,
    video_writer: Box<dyn VideoWriter>,
    save_folder: PathBuf,
    current_object_id: Option<u32>,
    frames_after_trigger: usize,
}

#[derive(Clone)]
pub struct ImageData {
    pub data: ImageBuffer<Luma<u8>, Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub nframe: u32,
    pub acq_nframe: u32,
    pub timestamp_raw: u64,
    pub exposure_time: u32,
}

impl FrameHandler {
    pub fn new(
        n_before: usize,
        n_after: usize,
        video_writer: Box<dyn VideoWriter>,
        save_folder: PathBuf,
    ) -> Self {
        Self {
            buffer: VecDeque::with_capacity(n_before + n_after),
            n_before,
            n_after,
            video_writer,
            save_folder,
            current_object_id: None,
            frames_after_trigger: 0,
        }
    }

    pub fn handle_frame(&mut self, frame: Arc<ImageData>, message: Option<Message>) -> Result<(), Error> {
        self.buffer.push_back(frame.clone());

        if self.buffer.len() > self.n_before + self.n_after {
            self.buffer.pop_front();
        }

        if let Some(msg) = message {
            match msg.message_type {
                crate::messaging::MessageType::JsonData => {
                    // Assume the message content is a KalmanEstimateRow
                    let kalman_data: crate::messaging::KalmanEstimateRow = 
                        serde_json::from_str(&msg.content)
                            .map_err(|e| Error::ParseError(format!("Failed to parse KalmanEstimateRow: {}", e)))?;
                    
                    self.handle_trigger(kalman_data.obj_id)?;
                }
                crate::messaging::MessageType::Kill => {
                    self.finish_current_video()?;
                }
                _ => {}
            }
        }

        if self.current_object_id.is_some() {
            self.video_writer.write_frame(&frame.data)?;
            self.frames_after_trigger += 1;

            if self.frames_after_trigger >= self.n_after {
                self.finish_current_video()?;
            }
        }

        Ok(())
    }

    fn handle_trigger(&mut self, obj_id: u32) -> Result<(), Error> {
        self.finish_current_video()?;

        self.current_object_id = Some(obj_id);
        self.frames_after_trigger = 0;

        let video_path = self.save_folder.join(format!("obj_id_{}_frame_{}.mp4", obj_id, self.buffer.back().map_or(0, |f| f.nframe)));
        self.video_writer = Box::new(FfmpegWriter::new(video_path)?);

        // Write buffered frames
        for frame in &self.buffer {
            self.video_writer.write_frame(&frame.data)?;
        }

        Ok(())
    }

    fn finish_current_video(&mut self) -> Result<(), Error> {
        if self.current_object_id.is_some() {
            self.video_writer.finish()?;
            self.current_object_id = None;
            self.frames_after_trigger = 0;
        }
        Ok(())
    }
}