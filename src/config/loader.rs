use anyhow::{Context, Result};
use log::{info, warn};
use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::camera::CameraConfig;
use crate::cli::CliArgs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub camera: CameraConfig,
    pub buffer: BufferConfig,
    pub network: NetworkConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Deserialize)]
pub struct BufferConfig {
    pub t_before: f32,
    pub t_after: f32,
}

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    pub address: String,
    pub sub_port: String,
    pub req_port: String,
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub save_folder: String,
}

impl Config {
    pub fn load(cli_args: &CliArgs) -> Result<Self> {
        let config_path = cli_args.config.as_deref().unwrap_or("config/default.toml");
        info!("Loading configuration from {}", config_path);

        let config_str = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path))?;

        let mut config: Config = toml::from_str(&config_str)
            .with_context(|| format!("Failed to parse config file: {}", config_path))?;

        // Override config with CLI arguments
        config.override_with_cli_args(cli_args);

        config.validate()?;

        Ok(config)
    }

    fn override_with_cli_args(&mut self, args: &CliArgs) {
        // Override camera config
        if args.serial != 0 {
            self.camera.serial = args.serial;
        }
        if args.fps != 500.0 {
            self.camera.fps = args.fps;
        }
        if args.exposure != 2000.0 {
            self.camera.exposure = args.exposure;
        }
        if args.width != 2016 {
            self.camera.width = args.width;
        }
        if args.height != 2016 {
            self.camera.height = args.height;
        }
        if args.offset_x != 1056 {
            self.camera.offset_x = args.offset_x;
        }
        if args.offset_y != 170 {
            self.camera.offset_y = args.offset_y;
        }

        // Override buffer config
        if args.t_before != 0.5 {
            self.buffer.t_before = args.t_before;
        }
        if args.t_after != 1.0 {
            self.buffer.t_after = args.t_after;
        }

        // Override network config
        if args.address != "127.0.0.1" {
            self.network.address = args.address.clone();
        }
        if args.sub_port != "5556" {
            self.network.sub_port = args.sub_port.clone();
        }
        if args.req_port != "5557" {
            self.network.req_port = args.req_port.clone();
        }

        // Override output config
        if args.save_folder != "output" {
            self.output.save_folder = args.save_folder.clone();
        }
    }

    fn validate(&self) -> Result<()> {
        // Validate camera config
        self.camera.validate().context("Invalid camera configuration")?;

        // Validate buffer config
        if self.buffer.t_before <= 0.0 || self.buffer.t_after <= 0.0 {
            return Err(anyhow::anyhow!("Buffer times must be positive"));
        }

        // Validate network config
        if self.network.sub_port.is_empty() || self.network.req_port.is_empty() {
            return Err(anyhow::anyhow!("Network ports cannot be empty"));
        }

        // Validate output config
        if self.output.save_folder.is_empty() {
            return Err(anyhow::anyhow!("Save folder cannot be empty"));
        }

        // Ensure save folder exists
        let save_folder = Path::new(&self.output.save_folder);
        if !save_folder.exists() {
            warn!("Save folder does not exist. Creating it.");
            fs::create_dir_all(save_folder)
                .with_context(|| format!("Failed to create save folder: {}", self.output.save_folder))?;
        }

        Ok(())
    }
}