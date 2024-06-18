// External crate imports, alphabetized
use clap::Parser;
use image::{ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeError;

// Standard library imports, alphabetized
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, default_value_t = 0)]
    pub serial: u32,

    #[arg(long, default_value_t = 500.0)]
    pub fps: f32,

    #[arg(long, default_value_t = 1200.0)]
    pub exposure: f32,

    #[arg(long, default_value_t = 10.0)]
    pub aperture: f32,

    #[arg(long, default_value_t = 2016)]
    pub width: u32,

    #[arg(long, default_value_t = 2016)]
    pub height: u32,

    #[arg(long, default_value_t = 1216)]
    pub offset_x: u32,

    #[arg(long, default_value_t = 126)]
    pub offset_y: u32,

    #[arg(long, default_value_t = 0.5)]
    pub t_before: f32,

    #[arg(long, default_value_t = 1.0)]
    pub t_after: f32,

    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    pub address: String,

    #[arg(long, default_value_t = String::from("5556"))]
    pub sub_port: String,

    #[arg(long, default_value_t = String::from("5557"))]
    pub req_port: String,

    #[arg(long, default_value_t = false)]
    pub debug: bool,

    #[arg(long, default_value_t = String::from("None"))]
    pub save_folder: String,
}

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

#[allow(dead_code)]
pub enum Packet {
    Images(Vec<Arc<ImageData>>),
    Kill,
}
