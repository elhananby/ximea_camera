// External crate imports, alphabetized
use clap::Parser;

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