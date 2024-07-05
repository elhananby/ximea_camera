// External crate imports, alphabetized
use image::{ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeError;
use std::collections::VecDeque;
use std::path::PathBuf;

// Standard library imports, alphabetized
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct ImageData {
    pub data: ImageBuffer<Luma<u8>, Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub nframe: u32,
    pub acq_nframe: u32,
    pub timestamp_raw: u64,
    pub exposure_time: u32,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone)]
#[serde(default)]
pub struct KalmanEstimateRow {
    pub obj_id: u32,
    pub frame: u64,
    pub timestamp: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub xvel: f64,
    pub yvel: f64,
    pub zvel: f64,
    pub P00: f64,
    pub P01: f64,
    pub P02: f64,
    pub P11: f64,
    pub P12: f64,
    pub P22: f64,
    pub P33: f64,
    pub P44: f64,
    pub P55: f64,
}

// Adjusted for the enum
#[derive(Debug)]
pub enum MessageType {
    Empty,
    JsonData(KalmanEstimateRow),
    Text(String),
    InvalidJson(String, SerdeError), // New variant to include parsing error details
}

pub struct FramesPacket {
    pub images: VecDeque<Arc<ImageData>>,
    pub save_path: PathBuf,
}
