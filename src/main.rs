use clap::Parser;
use crossbeam::channel;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use anyhow::{Result, Context};
use ctrlc;

mod camera;
mod frames;
mod messages;
mod structs;

use camera::*;
use frames::frame_handler;
use messages::{connect_to_socket, parse_message, subscribe_to_messages};
use structs::*;

fn main() -> Result<()> {
    // Set up logging
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // Parse command line arguments
    let args = Args::parse();

    // Set up ctrl-c handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).context("Error setting Ctrl-C handler")?;

    // Open the camera
    let mut cam = xiapi::open_device(Some(0)).map_err(|e| anyhow::anyhow!(e))?;

    // Set camera parameters
    set_camera_parameters(&mut cam, &args).context("Failed to set camera parameters")?;

    // Calculate frames before and after
    let n_before = (args.t_before * args.fps) as usize;
    let n_after = (args.t_after * args.fps) as usize;
    log::info!("Recording {} frames before and {} after trigger", n_before, n_after);

    // Connect to ZMQ
    log::info!("Connecting to ZMQ server at {}", args.address);
    let handshake = connect_to_socket(&args.req_port, zmq::REQ).context("Failed to connect to ZMQ REQ socket")?;

    // Send ready message to ZMQ over REQ
    handshake.send("Hello", 0).context("Failed to send handshake message")?;

    match handshake.recv_string(0) {
        Ok(Ok(msg)) if msg == "Welcome" => log::info!("Handshake successful"),
        _ => return Err(anyhow::anyhow!("Handshake failed")),
    }

    // Connect to ZMQ subscriber
    let subscriber = connect_to_socket(&args.sub_port, zmq::SUB).context("Failed to connect to ZMQ SUB socket")?;

    // Set up channels and spawn threads
    let (sender, receiver) = channel::unbounded();
    let frame_handler_thread = thread::spawn(move || frame_handler(receiver, n_before, n_after, args.save_folder));

    let (msg_sender, msg_receiver) = channel::unbounded();
    let subscriber_thread = thread::spawn(move || subscribe_to_messages(subscriber, msg_sender));

    // Start acquisition
    let buffer = cam.start_acquisition().map_err(|e| anyhow::anyhow!(e))?;
    log::info!("Starting acquisition");

    while running.load(Ordering::SeqCst) {
        if let Ok(message) = msg_receiver.try_recv() {
            let parsed_message = parse_message(&message);
            log::debug!("Parsed message: {:?}", parsed_message);

            if let MessageType::Text(data) = &parsed_message {
                if data == "kill" {
                    break;
                }
            }
        }

        let frame = buffer.next_image::<u8>(None).map_err(|e| anyhow::anyhow!("Camera error: {}", e)).context("Failed to get next frame")?;

        let image_data = Arc::new(ImageData {
            width: frame.width(),
            height: frame.height(),
            nframe: frame.nframe(),
            acq_nframe: frame.acq_nframe(),
            timestamp_raw: frame.timestamp_raw(),
            exposure_time: frame.exposure_time_us(),
            data: frame.into(),
        });

        if sender.send((image_data, MessageType::Empty)).is_err() {
            log::warn!("Failed to send frame to frame handler");
        }
    }

    // Clean up
    buffer.stop_acquisition().map_err(|e| anyhow::anyhow!("Camera error: {}", e)).context("Failed to stop acquisition")?;
    sender.send((Arc::new(ImageData::default()), MessageType::Text("kill".to_string())))
        .context("Failed to send kill message to frame handler")?;

    frame_handler_thread.join().map_err(|e| anyhow::anyhow!("Thread panicked: {:?}", e))?.context("Failed to join frame handler thread")?;
    subscriber_thread.join().map_err(|e| anyhow::anyhow!("Thread panicked: {:?}", e))?.context("Failed to join subscriber thread")?;

    Ok(())
}