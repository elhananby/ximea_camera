use log::{LevelFilter, debug, info, warn, error};
use fern::colors::{Color, ColoredLevelConfig};
use chrono::Local;
use std::io;
use anyhow::Result;

pub fn setup_logging(verbosity: u8, log_file: Option<&str>) -> Result<()> {
    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Green)
        .debug(Color::Blue)
        .trace(Color::Magenta);

    let mut base_config = fern::Dispatch::new();

    base_config = match verbosity {
        0 => base_config.level(LevelFilter::Info),
        1 => base_config.level(LevelFilter::Debug),
        _ => base_config.level(LevelFilter::Trace),
    };

    // Separate file config so we can include year, month and day in file logs
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        });

    let stdout_config = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                Local::now().format("[%H:%M:%S]"),
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .chain(io::stdout());

    base_config = base_config.chain(stdout_config);

    if let Some(log_file) = log_file {
        base_config = base_config.chain(
            file_config.chain(fern::log_file(log_file)?)
        );
    }

    base_config.apply()?;

    info!("Logging system initialized");
    debug!("Debug logging enabled");
    warn!("This is a test warning message");
    error!("This is a test error message");

    Ok(())
}

pub fn log_app_start(version: &str) {
    info!("Starting XIMEA Camera Application v{}", version);
}

pub fn log_app_config(config: &crate::config::Config) {
    info!("Application configured with:");
    info!("  Camera:");
    info!("    Serial: {}", config.camera.serial);
    info!("    FPS: {}", config.camera.fps);
    info!("    Exposure: {}", config.camera.exposure);
    info!("    Resolution: {}x{}", config.camera.width, config.camera.height);
    info!("    Offset: ({}, {})", config.camera.offset_x, config.camera.offset_y);
    info!("  Buffer:");
    info!("    Time before trigger: {}", config.buffer.t_before);
    info!("    Time after trigger: {}", config.buffer.t_after);
    info!("  Network:");
    info!("    Address: {}", config.network.address);
    info!("    Subscriber port: {}", config.network.sub_port);
    info!("    Request port: {}", config.network.req_port);
    info!("  Output:");
    info!("    Save folder: {}", config.output.save_folder);
}