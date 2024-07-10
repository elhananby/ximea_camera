use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing::{error, info};

mod camera;
mod communication;
mod processing;
mod types;
mod utils;

use std::sync::Mutex;

use camera::XiCamera;
use communication::ZmqClient;
use processing::FrameProcessor;
use types::SystemEvent;
use utils::{init_logging, AppError, Config};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Parse command-line arguments
    let args = Args::parse();

    // Load configuration
    let config = Config::load(&args.config).map_err(|e| AppError::ConfigError(e.to_string()))?;

    // Initialize logging
    init_logging(&config.log).map_err(|e| AppError::Other(e.to_string()))?;

    // Log startup information
    info!("Starting High-Speed Camera Frame Capture and Processing System");
    info!("Configuration loaded from: {}", args.config);

    // Initialize camera
    let camera = XiCamera::new(&config.camera).map_err(|e| AppError::CameraError(e.to_string()))?;
    let camera = Arc::new(Mutex::new(camera));

    // Initialize ZMQ client
    let zmq_client = ZmqClient::new(&config.zmq);

    // Set up channels
    let (frame_sender, frame_receiver) = tokio::sync::mpsc::channel(config.processing.buffer_size);
    let (trigger_sender, trigger_receiver) = tokio::sync::mpsc::channel(100);
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::channel(100);

    // Initialize frame processor
    let mut frame_processor = FrameProcessor::new(&config.processing, event_sender.clone())
        .map_err(|e| AppError::ProcessingError(e.to_string()))?;

    // Spawn frame processing task
    let processing_handle =
        tokio::spawn(async move { frame_processor.run(frame_receiver, trigger_receiver).await });

    // Spawn ZMQ listening task
    let zmq_handle = tokio::spawn(async move {
        if let Err(e) = zmq_client.listen_for_triggers(trigger_sender).await {
            error!("ZMQ listener error: {}", e);
        }
    });

    // Main capture loop
    info!("Starting main capture loop");
    camera
        .lock()
        .unwrap()
        .start_acquisition()
        .map_err(|e| AppError::CameraError(e.to_string()))?;

    let mut shutdown_requested = false;
    while !shutdown_requested {
        let camera_clone = Arc::clone(&camera);
        tokio::select! {
            frame_result = tokio::task::spawn_blocking(move || {
                camera_clone.lock().unwrap().capture_frame()
            }) => {
                match frame_result {
                    Ok(Ok(frame)) => {
                        if let Err(e) = frame_sender.send(Arc::new(frame)).await {
                            error!("Failed to send frame: {}", e);
                            shutdown_requested = true;
                        }
                    }
                    Ok(Err(e)) => {
                        error!("Failed to capture frame: {}", e);
                        shutdown_requested = true;
                    }
                    Err(e) => {
                        error!("Frame capture task panicked: {}", e);
                        shutdown_requested = true;
                    }
                }
            }
            Some(event) = event_receiver.recv() => {
                match event {
                    SystemEvent::VideoSaved(metadata) => {
                        info!("Video saved: {:?}", metadata);
                    }
                    SystemEvent::Error(err) => {
                        error!("System error: {}", err);
                        shutdown_requested = true;
                    }
                    _ => {}  // Handle other events as needed
                }
            }

            _ = tokio::signal::ctrl_c() => {
                info!("Received interrupt signal, initiating shutdown");
                shutdown_requested = true;
            }
        }
    }

    // Shutdown procedure
    info!("Stopping camera acquisition");
    camera
        .lock()
        .unwrap()
        .stop_acquisition()
        .map_err(|e| AppError::CameraError(e.to_string()))?;

    info!("Waiting for frame processor to finish");
    if let Err(e) = processing_handle.await {
        error!("Frame processor panicked: {:?}", e);
    }

    info!("Waiting for ZMQ client to finish");
    if let Err(e) = zmq_handle.await {
        error!("ZMQ client panicked: {:?}", e);
    }

    info!("Shutdown complete");
    Ok(())
}
