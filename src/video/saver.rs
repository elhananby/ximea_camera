use anyhow::{Context, Result};
use log::{info, error, debug};
use std::process::{Command, Stdio, Child};
use std::io::Write;
use crate::frame::Frame;

pub struct VideoSaver {
    ffmpeg_process: Child,
    width: u32,
    height: u32,
    fps: u32,
    output_path: String,
}

impl VideoSaver {
    pub fn new(width: u32, height: u32, fps: u32, output_path: &str) -> Result<Self> {
        let ffmpeg_process = Command::new("ffmpeg")
            .args(&[
                "-f", "rawvideo",
                "-pixel_format", "gray",
                "-video_size", &format!("{}x{}", width, height),
                "-framerate", &fps.to_string(),
                "-i", "-",
                "-c:v", "libx264",
                "-preset", "ultrafast",
                "-y",
                output_path,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start ffmpeg process")?;

        info!("Started ffmpeg process for video saving");

        Ok(Self {
            ffmpeg_process,
            width,
            height,
            fps,
            output_path: output_path.to_string(),
        })
    }

    pub fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        let stdin = self.ffmpeg_process.stdin.as_mut()
            .context("Failed to get stdin of ffmpeg process")?;

        stdin.write_all(&frame.data)
            .context("Failed to write frame data to ffmpeg")?;

        debug!("Wrote frame {} to video", frame.nframe);

        Ok(())
    }

    pub fn finalize(mut self) -> Result<()> {
        // Close stdin to signal end of input to ffmpeg
        drop(self.ffmpeg_process.stdin.take());

        let output = self.ffmpeg_process.wait_with_output()
            .context("Failed to wait for ffmpeg process")?;

        if output.status.success() {
            info!("Successfully saved video to {}", self.output_path);
            Ok(())
        } else {
            let error_message = String::from_utf8_lossy(&output.stderr);
            error!("FFmpeg error: {}", error_message);
            Err(anyhow::anyhow!("FFmpeg process failed"))
        }
    }
}

impl Drop for VideoSaver {
    fn drop(&mut self) {
        if let Err(e) = self.ffmpeg_process.kill() {
            error!("Failed to kill ffmpeg process: {}", e);
        }
    }
}