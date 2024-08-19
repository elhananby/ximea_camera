mod frame_handler;
mod ffmpeg_writer;

pub use frame_handler::FrameHandler;
pub use ffmpeg_writer::FfmpegWriter;

pub trait VideoWriter {
    fn write_frame(&mut self, frame: &[u8]) -> Result<(), crate::error::Error>;
    fn finish(&mut self) -> Result<(), crate::error::Error>;
}