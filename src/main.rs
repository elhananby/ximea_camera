use std::sync::Arc;
use std::thread;
use crossbeam::channel;

mod camera;
mod messaging;
mod video;
mod config;
mod error;
mod utils;

use camera::{open_camera, XimeaCamera};
use messaging::{ZmqHandler, MessageHandler, Message};
use video::{FrameHandler, FfmpegWriter};
use config::{Config, CliArgs};
use error::Error;

fn main() -> Result<(), Error> {
    // Initialize logging
    env_logger::init();

    // Parse command line arguments and create config
    let cli_args = CliArgs::parse();
    let config = Config::from_cli(cli_args)?;

    log::info!("Starting application with config: {:?}", config);

    // Initialize camera
    let camera = open_camera(config.camera)?;
    let acquisition_buffer = camera.start_acquisition()?;

    // Initialize ZMQ handler
    let mut zmq_handler = ZmqHandler::new(config.messaging.sub_port.clone(), config.messaging.req_port.clone());
    zmq_handler.connect()?;

    // Initialize video writer and frame handler
    let video_writer = Box::new(FfmpegWriter::new(std::path::PathBuf::from(&config.video.save_folder))?);
    let mut frame_handler = FrameHandler::new(
        (config.video.t_before * config.camera.fps as f32) as usize,
        (config.video.t_after * config.camera.fps as f32) as usize,
        video_writer,
        std::path::PathBuf::from(config.video.save_folder),
    );

    // Create channels for communication between threads
    let (frame_sender, frame_receiver) = channel::unbounded();
    let (message_sender, message_receiver) = channel::unbounded();

    // Spawn camera capture thread
    let camera_thread = thread::spawn(move || -> Result<(), Error> {
        loop {
            let image = acquisition_buffer.next_image::<u8>(None)
                .map_err(|e| Error::CameraError(format!("Failed to capture image: {}", e)))?;
            let frame_data = Arc::new(image.data().to_vec());
            if frame_sender.send(frame_data).is_err() {
                break;
            }
        }
        Ok(())
    });

    // Spawn message handling thread
    let message_thread = thread::spawn(move || -> Result<(), Error> {
        loop {
            if let Some(message) = zmq_handler.receive_message()? {
                if message_sender.send(message).is_err() {
                    break;
                }
            }
        }
        Ok(())
    });

    // Main processing loop
    loop {
        crossbeam::select! {
            recv(frame_receiver) -> frame => {
                let frame = frame?;
                let message = message_receiver.try_recv().ok();
                frame_handler.handle_frame(frame, message)?;
            }
            recv(message_receiver) -> message => {
                let message = message?;
                if let Message::Kill = message {
                    log::info!("Received kill message, shutting down...");
                    break;
                }
            }
        }
    }

    // Cleanup
    drop(frame_sender);
    drop(message_sender);
    camera_thread.join().unwrap()?;
    message_thread.join().unwrap()?;

    log::info!("Application shut down successfully");
    Ok(())
}