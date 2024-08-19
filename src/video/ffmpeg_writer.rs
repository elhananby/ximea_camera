use super::VideoWriter;
use crate::error::Error;
use std::path::PathBuf;
use std::process::{Command, Stdio, Child};
use std::io::Write;

pub struct FfmpegWriter {
    ffmpeg_process: Child,
    width: u32,
    height: u32,
}

impl FfmpegWriter {
    pub fn new(output_path: PathBuf) -> Result<Self, Error> {
        // Assuming a fixed width and height for now. In a real application,
        // you'd want to pass these as parameters or determine them from the first frame.
        let width = 2016;
        let height = 2016;

        let ffmpeg_process = Command::new("ffmpeg")
            .args([
                "-f", "rawvideo",
                "-pixel_format", "gray",
                "-video_size", &format!("{}x{}", width, height),
                "-framerate", "25",
                "-i", "-",
                "-vf", "format=gray",
                "-vcodec", "h264_nvenc",
                "-preset", "p4",
                "-tune", "hq",
                output_path.to_str().unwrap(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::FFmpegError(format!("Failed to start FFmpeg process: {}", e)))?;

        Ok(Self {
            ffmpeg_process,
            width,
            height,
        })
    }
}

impl VideoWriter for FfmpegWriter {
    fn write_frame(&mut self, frame: &[u8]) -> Result<(), Error> {
        if let Some(stdin) = self.ffmpeg_process.stdin.as_mut() {
            stdin.write_all(frame)
                .map_err(|e| Error::FFmpegError(format!("Failed to write frame to FFmpeg: {}", e)))?;
        } else {
            return Err(Error::FFmpegError("FFmpeg process stdin not available".to_string()));
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Error> {
        // Close stdin to signal FFmpeg that we're done
        self.ffmpeg_process.stdin.take();
        
        // Wait for FFmpeg to finish processing
        let output = self.ffmpeg_process.wait()
            .map_err(|e| Error::FFmpegError(format!("Failed to wait for FFmpeg process: {}", e)))?;

        if !output.success() {
            return Err(Error::FFmpegError(format!("FFmpeg process exited with non-zero status: {:?}", output)));
        }

        Ok(())
    }
}