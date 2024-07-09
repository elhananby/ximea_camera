use anyhow::{Result, Context};
use std::path::Path;
use std::process::{Command, Stdio};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tracing::{info, error};

use crate::camera::Frame;
use crate::utils::config::VideoConfig;

pub struct VideoWriter {
    config: VideoConfig,
}

impl VideoWriter {
    pub fn new(config: &VideoConfig) -> Result<Self> {
        Ok(VideoWriter {
            config: config.clone(),
        })
    }

    pub async fn write_video(&self, path: &Path, frames: &VecDeque<Arc<Frame>>) -> Result<()> {
        info!("Writing video to {:?}", path);

        let mut ffmpeg = Command::new("ffmpeg")
            .args(&[
                "-f", "rawvideo",
                "-pixel_format", "gray",
                "-video_size", &format!("{}x{}", frames[0].width, frames[0].height),
                "-framerate", &self.config.framerate.to_string(),
                "-i", "-",
                "-c:v", "libx264",
                "-preset", "ultrafast",
                "-crf", "23",
                "-y",
                path.to_str().unwrap(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn FFmpeg process")?;

        let mut stdin = ffmpeg.stdin.take()
            .context("Failed to open FFmpeg stdin")?;

        let mut stdin = tokio::process::ChildStdin::from_std(stdin)
            .context("Failed to create tokio ChildStdin")?;

        for frame in frames {
            stdin.write_all(frame.as_bytes()).await
                .context("Failed to write frame to FFmpeg")?;
        }

        drop(stdin);

        let status = ffmpeg.wait()
            .context("Failed to wait for FFmpeg process")?;

        if !status.success() {
            error!("FFmpeg process failed with status: {}", status);
            anyhow::bail!("FFmpeg process failed");
        }

        info!("Video writing complete");
        Ok(())
    }
}