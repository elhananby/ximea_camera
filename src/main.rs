// External crate imports
use clap::Parser;
use crossbeam::channel;
use image::{ImageBuffer, Luma};

use std::sync::Arc;

// Local module declarations
mod frames;
mod helpers;
mod structs;

// Imports from local modules
use frames::frame_handler;
use helpers::*;

use structs::*;
fn main() -> Result<(), i32> {
    // set logging level
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }

    // setup logger
    env_logger::init();

    // setup ctrl-c handler
    // let running = Arc::new(AtomicBool::new(true));
    // setup_ctrlc_handler(running.clone());

    // Parse command line arguments
    let args = Args::parse();

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
    log::info!("Connecting to ZMQ server at {}", args.address);
    let handshake = connect_to_socket(&args.req_port, zmq::REQ);

    // Send ready message to ZMQ over REQ
    log::info!("Sending ready message to ZMQ PUB");
    handshake.send("Hello", 0).unwrap();
    let message = handshake.recv_string(0);
    println!("Received message: {:?}", message);

    match handshake.recv_string(0) {
        Ok(Ok(msg)) if &msg == "Welcome " => {
            log::info!("Handshake successfull");
        }
        Ok(Err(e)) => {
            log::error!("Failed to receive message: {:?}", e);
        }
        Err(e) => {
            log::error!("Failed to receive message: {}", e);
        }
        Ok(_) => {
            log::error!("Handshake failed");
            return Err(1);
        }
    }

    let subscriber = connect_to_socket(&args.sub_port, zmq::SUB);

    // Wait for ready message from socket
    log::info!("Waiting for ready message from ZMQ PUB");

    // Block until first message, which should be the save folder
    // subscriber.recv(&mut msg, 0).unwrap();
    let save_folder = args.save_folder.clone();

    // spawn writer thread
    let (sender, receiver) = channel::unbounded::<(Arc<ImageData>, MessageType)>();
    let frame_handler =
        std::thread::spawn(move || frame_handler(receiver, n_before, n_after, save_folder));

    // create image buffer
    let buffer = cam.start_acquisition()?;
    //let mut image_data: Arc<ImageData> = Arc::new(ImageData::default());

    // start acquisition
    log::info!("Starting acquisition");
    loop {
        // receive message
        let msg = match subscriber.recv_string(zmq::DONTWAIT) {
            Ok(Ok(full_message)) => {
                let parts: Vec<&str> = full_message.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let topic = parts[0];
                    let message = parts[1];
                    log::debug!("Received message: {:?} {:?}", topic, message);
                    Some(message.to_string())
                } else {
                    log::warn!("Received message with no topic: {:?}", full_message);
                    Some(full_message)
                }
            }
            Ok(Err(_)) => {
                log::debug!("Failed to receive message");
                None
            }
            Err(e) => {
                log::debug!("Failed to receive message: {:?}", e);
                None
            }
        };

        // parse message
        let mut parsed_message = MessageType::Empty;
        if let Some(message) = msg {
            parsed_message = parse_message(&message);
        } else {
            log::debug!("No valid message received.");
        }

        // check if got "kill" in parsed_message
        if let MessageType::Text(data) = &parsed_message {
            if data == "kill" {
                break;
            }
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
                log::debug!("Sent frame to frame handler");
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
    frame_handler.join().unwrap();

    Ok(())
}
