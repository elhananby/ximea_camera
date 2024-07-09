use tokio;
use clap::Parser;
use anyhow::Result;
use tracing::{info, error};
use std::sync::Arc;

mod camera;
mod processing;
mod communication;
mod utils;
mod types;

use camera::XiCamera;
use processing::FrameProcessor;
use communication::ZmqClient;
use utils::{Config, init_logging, AppError};
use types::{TriggerMessage, SystemEvent};

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
    let mut camera = XiCamera::new(&config.camera).map_err(|e| AppError::CameraError(e.to_string()))?;

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
    let processing_handle = tokio::spawn(async move {
        frame_processor.run(frame_receiver, trigger_receiver).await
    });

    // Spawn ZMQ listening task
    let zmq_handle = tokio::spawn(async move {
        if let Err(e) = zmq_client.listen_for_triggers(trigger_sender).await {
            error!("ZMQ listener error: {}", e);
        }
    }); 

    // Main capture loop
    info!("Starting main capture loop");
    camera.start_acquisition().map_err(|e| AppError::CameraError(e.to_string()))?;
    
    let mut shutdown_requested = false;
    while !shutdown_requested {
        tokio::select! {
            _ = tokio::task::spawn_blocking(move || camera.capture_frame()) => {
                match camera.capture_frame() {
                    Ok(frame) => {
                        if let Err(e) = frame_sender.send(Arc::new(frame)).await {
                            error!("Failed to send frame: {}", e);
                            shutdown_requested = true;
                        }
                    }
                    Err(e) => {
                        error!("Failed to capture frame: {}", e);
                        shutdown_requested = true;
                    }
                }
            }
            Some(event) = event_receiver.recv() => {
                // ... (event handling remains the same)
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received interrupt signal, initiating shutdown");
                shutdown_requested = true;
            }
        }
    }   

    // Shutdown procedure
    info!("Stopping camera acquisition");
    camera.stop_acquisition().map_err(|e| AppError::CameraError(e.to_string()))?;

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