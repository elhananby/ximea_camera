use anyhow::{Context, Result};
use clap::Parser;
use crossbeam::channel;
use image::{ImageBuffer, Luma};
use std::sync::Arc;
use tokio::signal;
use tracing::{debug, error, info, warn};

mod camera;
mod frames;
mod messages;
mod structs;

use camera::*;
use frames::frame_handler;
use messages::{connect_to_socket, parse_message, subscribe_to_messages};
use structs::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();

    // Open the camera
    let mut cam = xiapi::open_device(Some(0))
        .map_err(|e| anyhow::anyhow!("Failed to open camera: error code {}", e))?;

    // Set camera parameters
    set_camera_parameters(&mut cam, &args)
        .map_err(|e| anyhow::anyhow!("Failed to set camera parameters: error code {}", e))?;

    // Calculate frames before and after
    let n_before = (args.t_before * args.fps) as usize;
    let n_after = (args.t_after * args.fps) as usize;
    debug!("Recording {} frames before and {} after trigger", n_before, n_after);

    // Connect to ZMQ
    info!("Connecting to ZMQ server at {}", args.address);
    let handshake = connect_to_socket(&args.req_port, zmq::REQ).await
        .context("Failed to connect to ZMQ REQ socket")?;

    // Send ready message to ZMQ over REQ
    info!("Sending ready message to ZMQ PUB");
    handshake.send("Hello", 0).context("Failed to send handshake message")?;

    match handshake.recv_string(0) {
        Ok(Ok(msg)) if &msg == "Welcome" => {
            info!("Handshake successful");
        }
        _ => {
            error!("Handshake failed");
            return Err(anyhow::anyhow!("Handshake failed"));
        }
    }

    // Connect to ZMQ subscriber
    let subscriber = connect_to_socket(&args.sub_port, zmq::SUB).await
        .context("Failed to connect to ZMQ SUB socket")?;

    // Set save folder
    let save_folder = args.save_folder.clone();

    // Spawn writer thread
    let (sender, receiver) = channel::unbounded::<(Arc<ImageData>, MessageType)>();
    let frame_handler_handle = tokio::spawn(async move {
        if let Err(e) = frame_handler(receiver, n_before, n_after, save_folder).await {
            error!("Error in frame handler: {}", e);
        }
    });

    // Spawn subscriber thread
    let (msg_sender, msg_receiver) = channel::unbounded::<String>();
    let subscriber_handle = tokio::spawn(async move {
        if let Err(e) = subscribe_to_messages(subscriber, msg_sender).await {
            error!("Error in subscriber: {}", e);
        }
    });

    // Create image buffer
    let buffer = cam.start_acquisition().context("Failed to start camera acquisition")?;

    // Start acquisition
    info!("Starting acquisition");
    let mut shutdown = false;
    while !shutdown {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Received Ctrl+C, initiating shutdown");
                shutdown = true;
            }
            _ = async {
                // Check if there's a message from the subscriber thread
                let msg = msg_receiver.try_recv().ok();

                // Parse message
                let parsed_message = msg.as_ref().map(|m| parse_message(m)).unwrap_or(MessageType::Empty);
                if let MessageType::Text(data) = &parsed_message {
                    if data == "kill" {
                        info!("Received kill message");
                        return Ok(());
                    }
                }

                // Get frame from camera
                let frame = buffer.next_image::<u8>(None).context("Failed to get next image")?;

                // Put frame data to struct
                let image_data = Arc::new(ImageData {
                    width: frame.width(),
                    height: frame.height(),
                    nframe: frame.nframe(),
                    acq_nframe: frame.acq_nframe(),
                    timestamp_raw: frame.timestamp_raw(),
                    exposure_time: frame.exposure_time_us(),
                    data: ImageBuffer::<Luma<u8>, Vec<u8>>::from(frame),
                });

                // Send frame with the incoming parsed message
                if let Err(e) = sender.send((image_data, parsed_message)) {
                    warn!("Failed to send frame to frame handler: {}", e);
                }

                Ok::<(), anyhow::Error>(())
            } => {}
        }
    }

    // Stop acquisition
    buffer.stop_acquisition().context("Failed to stop camera acquisition")?;

    // Send kill signal to writer thread
    sender.send((
        Arc::new(ImageData::default()),
        MessageType::Text("kill".to_string()),
    )).context("Failed to send kill message to frame handler")?;

    // Wait for frame handler and subscriber to finish
    frame_handler_handle.await??;
    subscriber_handle.await??;

    Ok(())
}