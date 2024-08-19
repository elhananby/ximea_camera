mod cli;

use crate::error::Error;

pub use cli::CliArgs;

#[derive(Debug, Clone)]
pub struct Config {
    pub camera: CameraConfig,
    pub messaging: MessagingConfig,
    pub video: VideoConfig,
}

#[derive(Debug, Clone)]
pub struct CameraConfig {
    pub serial: u32,
    pub fps: f32,
    pub exposure: f32,
    pub width: u32,
    pub height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
}

#[derive(Debug, Clone)]
pub struct MessagingConfig {
    pub address: String,
    pub sub_port: String,
    pub req_port: String,
}

#[derive(Debug, Clone)]
pub struct VideoConfig {
    pub t_before: f32,
    pub t_after: f32,
    pub save_folder: String,
}

impl Config {
    pub fn from_cli(args: CliArgs) -> Result<Self, Error> {
        Ok(Self {
            camera: CameraConfig {
                serial: args.serial,
                fps: args.fps,
                exposure: args.exposure,
                width: args.width,
                height: args.height,
                offset_x: args.offset_x,
                offset_y: args.offset_y,
            },
            messaging: MessagingConfig {
                address: args.address,
                sub_port: args.sub_port,
                req_port: args.req_port,
            },
            video: VideoConfig {
                t_before: args.t_before,
                t_after: args.t_after,
                save_folder: args.save_folder,
            },
        })
    }
}