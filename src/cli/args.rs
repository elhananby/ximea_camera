use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    #[arg(long, default_value_t = 0)]
    pub serial: u32,

    #[arg(long, default_value_t = 500.0)]
    pub fps: f32,

    #[arg(long, default_value_t = 2000.0)]
    pub exposure: f32,

    #[arg(long, default_value_t = 2016)]
    pub width: u32,

    #[arg(long, default_value_t = 2016)]
    pub height: u32,

    #[arg(long, default_value_t = 1056)]
    pub offset_x: u32,

    #[arg(long, default_value_t = 170)]
    pub offset_y: u32,

    #[arg(long, default_value_t = 0.5)]
    pub t_before: f32,

    #[arg(long, default_value_t = 1.0)]
    pub t_after: f32,

    #[arg(long, default_value = "127.0.0.1")]
    pub address: String,

    #[arg(long, default_value = "5556")]
    pub sub_port: String,

    #[arg(long, default_value = "5557")]
    pub req_port: String,

    #[arg(long)]
    pub debug: bool,

    #[arg(long, default_value = "output")]
    pub save_folder: String,

    #[arg(long)]
    pub config: Option<String>,
}

impl CliArgs {
    pub fn parse() -> Self {
        Self::parse()
    }

    pub fn to_camera_config(&self) -> crate::camera::CameraConfig {
        crate::camera::CameraConfig {
            serial: self.serial,
            fps: self.fps,
            exposure: self.exposure,
            width: self.width,
            height: self.height,
            offset_x: self.offset_x,
            offset_y: self.offset_y,
        }
    }
}