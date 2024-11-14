// External crate imports
use clap::Parser;
use crossbeam::channel;
use image::{ImageBuffer, Luma};

use std::sync::Arc;
use std::thread;

// Local module declarations
mod camera;
mod cli;
mod frames;
mod messages;
mod structs;

// Imports from local modules
use camera::*;
use cli::Args;
use frames::frame_handler;
use messages::{connect_to_socket, parse_message, subscribe_to_messages};
use structs::*;

fn main() -> Result<(), i32> {
    // set logging level
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }

    // setup logger
    env_logger::init();

    // Parse command line arguments
    let args = Args::parse();

    log::debug!("Command-line arguments: {:?}", &args);

    // Open the camera
    let mut cam = xiapi::open_device(Some(0))?;

    // Set camera parameters
    set_camera_parameters(&mut cam, &args)?;

    // calculate frames before and after
    let n_before = (args.t_before * args.fps) as usize;
    let n_after = (args.t_after * args.fps) as usize;
    log::debug!(
        "Recording {} frames before and {} after trigger",
        n_before,
        n_after
    );

    // Connect to ZMQ; return error if connection fails
    log::debug!("Connecting to ZMQ server at {}", args.sub_port);

    // Connect to ZMQ subscriber
    let subscriber = connect_to_socket(&args.sub_port, zmq::SUB);

    // Set save folder
    let save_folder = args.save_folder.clone();

    // spawn writer thread
    let (sender, receiver) = channel::unbounded::<(Arc<ImageData>, MessageType)>();
    let frame_handler_thread =
        thread::spawn(move || frame_handler(receiver, n_before, n_after, save_folder));

    // spawn subscriber thread
    let (msg_sender, msg_receiver) = channel::unbounded::<String>();
    let subscriber_thread = thread::spawn(move || subscribe_to_messages(subscriber, msg_sender));

    // create image buffer
    let buffer = cam.start_acquisition()?;

    // start acquisition
    log::info!("Starting acquisition");
    loop {
        // Check if there's a message from the subscriber thread
        let msg = match msg_receiver.try_recv() {
            Ok(message) => Some(message),
            Err(_) => None,
        };

        // parse message
        let mut parsed_message = MessageType::Empty;
        if let Some(message) = msg {
            parsed_message = parse_message(&message);
            log::debug!("Parsed message: {:?}", parsed_message);

            // check if got "kill" in parsed_message
            if let MessageType::Text(data) = &parsed_message {
                if data == "kill" {
                    break;
                }
            }
        } else {
            log::debug!("No valid message received.");
        }

        // Get frame from camera
        let frame = buffer.next_image::<u8>(None)?;

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

        // send frame with the incoming parsed message
        match sender.send((image_data, parsed_message)) {
            Ok(_) => {
                log::trace!("Sent frame to frame handler");
            }
            Err(_e) => {
                log::warn!("Failed to send frame to frame handler");
            } //log::error!("Failed to send frame: {}", e),
        }
    }

    // stop acquisition
    buffer.stop_acquisition()?;

    // send kill signal to writer thread
    match sender.send((
        Arc::new(ImageData::default()),
        MessageType::Text("kill".to_string()),
    )) {
        Ok(_) => {
            log::info!("Sent kill message to frame handler");
        }
        Err(_e) => {
            log::error!("Failed to send kill trigger to frame handler.")
        }
    }

    // stop frame handler
    frame_handler_thread.join().unwrap();
    subscriber_thread.join().unwrap();

    Ok(())
}
