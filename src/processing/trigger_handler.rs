use std::path::PathBuf;
use crate::types::TriggerMessage;

pub struct TriggerHandler;

impl TriggerHandler {
    pub fn new() -> Self {
        TriggerHandler
    }

    pub fn generate_video_path(&self, trigger: &TriggerMessage) -> PathBuf {
        PathBuf::from(format!("output/obj_id_{}_frame_{}.mp4", trigger.obj_id, trigger.frame))
    }
}