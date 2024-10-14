use ximea_camera::{
    camera::XimeaCamera,
    cli::CliArgs,
    config::Config,
    error::AppError,
    frame::{Frame, FrameBuffer, FrameProcessor},
    logging,
    messaging::{ZmqSubscriber, MessageType},
    video::{VideoSaver, save_video_metadata},
};

use anyhow::Result;
use log::{info, error};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<()> {
    // Parse command-line arguments
    let cli_args = CliArgs::parse();

    // Setup logging
    logging::setup_logging(cli_args.debug as u8, None)?;
    logging::log_app_start(env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Config::load(&cli_args)?;
    logging::log_app_config(&config);

    // Initialize camera
    let mut camera = XimeaCamera::new(&config.camera)
        .map_err(|e| AppError::Camera(e))?;

    // Initialize frame buffer
    let buffer_size = (config.buffer.t_before * config.camera.fps) as usize;
    let mut frame_buffer = FrameBuffer::new(buffer_size);

    // Initialize ZMQ subscriber
    let subscriber = ZmqSubscriber::new(&config.network.address, &config.network.sub_port)?;

    // Start image acquisition
    let acquisition = camera.start_acquisition()?;

    // Main loop
    info!("Entering main acquisition loop");
    let mut is_recording = false;
    let mut frames_after_trigger = 0;
    let frames_to_save = (config.buffer.t_after * config.camera.fps) as usize;
    let mut video_saver: Option<VideoSaver> = None;
    let mut saved_frames: Vec<Frame> = Vec::new();

    loop {
        // Check for incoming messages
        match subscriber.receive_message(Duration::from_millis(1)) {
            Ok(MessageType::JsonData(data)) => {
                info!("Received trigger for object {}", data.obj_id);
                is_recording = true;
                frames_after_trigger = 0;
                
                let output_path = format!("{}/obj_id_{}_frame_{}.mp4", 
                    config.output.save_folder, data.obj_id, data.frame);
                
                video_saver = Some(VideoSaver::new(
                    config.camera.width,
                    config.camera.height,
                    config.camera.fps as u32,
                    &output_path
                )?);
                
                // Save buffered frames
                for frame in frame_buffer.iter() {
                    if let Some(saver) = video_saver.as_mut() {
                        saver.write_frame(frame)?;
                        saved_frames.push(frame.as_ref().clone());
                    }
                }
            },
            Ok(MessageType::Text(text)) if text == "kill" => {
                info!("Received kill signal, stopping acquisition");
                break;
            },
            Ok(_) => {}, // Ignore other message types
            Err(e) => {
                error!("Error receiving message: {}", e);
                // Decide whether to break or continue based on the error
            }
        }

        // Capture and process frame
        let raw_frame = acquisition.next_image::<u8>(None)?;
        let frame = FrameProcessor::process_frame(&raw_frame)?;

        // Add frame to buffer
        frame_buffer.push(Arc::new(frame.clone()));

        // If recording, save frame
        if is_recording {
            if let Some(saver) = video_saver.as_mut() {
                saver.write_frame(&frame)?;
                saved_frames.push(frame);
            }

            frames_after_trigger += 1;
            if frames_after_trigger >= frames_to_save {
                is_recording = false;
                if let Some(saver) = video_saver.take() {
                    saver.finalize()?;
                    info!("Finished saving video");

                    let metadata_path = format!("{}/obj_id_{}_frame_{}_metadata.csv", 
                        config.output.save_folder, saved_frames[0].nframe, saved_frames.last().unwrap().nframe);
                    save_video_metadata(&saved_frames, std::path::Path::new(&metadata_path))?;
                    info!("Saved metadata to {}", metadata_path);

                    saved_frames.clear();
                }
            }
        }
    }

    // Stop acquisition
    camera.stop_acquisition()?;
    info!("Acquisition stopped, application shutting down");

    Ok(())
}