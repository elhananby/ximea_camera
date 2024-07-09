use anyhow::{Result, Context};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{info, error, debug};
use std::collections::VecDeque;
use std::sync::Arc;

use crate::camera::Frame;
use crate::types::{TriggerMessage, SystemEvent};
use crate::utils::config::ProcessingConfig;
use super::trigger_handler::TriggerHandler;
use super::video_writer::VideoWriter;

pub struct FrameProcessor {
    config: ProcessingConfig,
    buffer: VecDeque<Arc<Frame>>,
    trigger_handler: TriggerHandler,
    video_writer: VideoWriter,
    state: ProcessorState,
    event_sender: Sender<SystemEvent>,
}

#[derive(Debug)]
enum ProcessorState {
    Buffering,
    Waiting,
    Recording(TriggerMessage),
    Saving,
}

impl FrameProcessor {
    pub fn new(config: &ProcessingConfig, event_sender: Sender<SystemEvent>) -> Result<Self> {
        Ok(FrameProcessor {
            config: config.clone(),
            buffer: VecDeque::with_capacity(config.buffer_size),
            trigger_handler: TriggerHandler::new(),
            video_writer: VideoWriter::new(&config.video_config)?,
            state: ProcessorState::Buffering,
            event_sender,
        })
    }

    pub async fn run(
        &mut self,
        mut frame_rx: Receiver<Arc<Frame>>,
        mut trigger_rx: Receiver<TriggerMessage>,
    ) -> Result<()> {
        info!("Frame processor started");

        loop {
            tokio::select! {
                Some(frame) = frame_rx.recv() => {
                    self.handle_frame(frame).await?;
                }
                Some(trigger) = trigger_rx.recv() => {
                    self.handle_trigger(trigger).await?;
                }
                else => break,
            }
        }

        info!("Frame processor stopped");
        Ok(())
    }

    async fn handle_frame(&mut self, frame: Arc<Frame>) -> Result<()> {
        match &self.state {
            ProcessorState::Buffering => {
                self.buffer.push_back(frame);
                if self.buffer.len() >= self.config.buffer_size {
                    self.state = ProcessorState::Waiting;
                }
            }
            ProcessorState::Waiting => {
                self.buffer.push_back(frame);
                if self.buffer.len() > self.config.buffer_size {
                    self.buffer.pop_front();
                }
            }
            ProcessorState::Recording(trigger) => {
                self.buffer.push_back(frame);
                if self.buffer.len() >= self.config.buffer_size + self.config.frames_after_trigger {
                    let trigger_clone = trigger.clone();
                    self.state = ProcessorState::Saving;
                    self.save_video(&trigger_clone).await?;
                }
            }
            ProcessorState::Saving => {
                // Drop frames while saving
                debug!("Dropping frame while saving video");
            }
        }
        Ok(())
    }

    async fn handle_trigger(&mut self, trigger: TriggerMessage) -> Result<()> {
        match &self.state {
            ProcessorState::Waiting => {
                info!("Trigger received, starting recording");
                self.state = ProcessorState::Recording(trigger);
            }
            _ => {
                debug!("Ignoring trigger in current state: {:?}", self.state);
            }
        }
        Ok(())
    }

    async fn save_video(&mut self, trigger: &TriggerMessage) -> Result<()> {
        info!("Saving video for trigger: {:?}", trigger);
        let video_path = self.trigger_handler.generate_video_path(trigger);
        self.video_writer.write_video(&video_path, &self.buffer).await?;
        self.buffer.clear();
        self.state = ProcessorState::Buffering;
        Ok(())
    }
}