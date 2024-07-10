use anyhow::{Context, Result};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::utils::config::LogConfig;

pub fn init(config: &LogConfig) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.level))
        .context("Failed to create EnvFilter")?;

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(env_filter)
        .with_ansi(true)
        .with_thread_names(true);

    if let Some(log_file) = &config.file {
        let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", log_file);
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        subscriber.with_writer(non_blocking).init();
    } else {
        subscriber.init();
    }

    Ok(())
}
