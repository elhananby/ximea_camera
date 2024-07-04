// Standard library imports
use std::{
    collections::VecDeque, fs::{create_dir_all, OpenOptions}, io::Write, path::{Path, PathBuf}, process::{Command, Stdio}, sync::Arc, thread, time::Instant
};

// External crates
use crossbeam::channel::{Receiver, unbounded};

use image::ImageFormat;
use rayon::prelude::*;

// Current crate
use crate::{
    structs::{ImageData, MessageType, FramesPacket},
    KalmanEstimateRow,
};


// fn save_images_to_disk(
//     images: &VecDeque<Arc<ImageData>>,
//     save_path: &Path,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     log::debug!("Saving images to disk");

//     // loop over images and save to disk
//     images.into_par_iter().for_each(|image| {
//         // set the filename to save the image to (based on the acq_nframe field of the image)
//         let filename = save_path.join(format!("{}.tiff", image.acq_nframe));

//         // save the image to disk
//         match image.data.save_with_format(&filename, ImageFormat::Tiff) {
//             // print a debug message if the image was saved successfully
//             Ok(_) => {}
//             // print an error message if the image failed to save
//             Err(e) => log::debug!("Failed to save {}: {}", filename.display(), e),
//         }
//     });

//     Ok(())
// }

fn save_video_metadata(
    images: &VecDeque<Arc<ImageData>>,
    save_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    log::debug!("Saving metadata to disk");
    // Open a file in write mode to save CSV data
    //let file = File::create(save_path.join("metadata.csv")).unwrap();
    let mut file = OpenOptions::new()
        .create_new(true)
        .append(true)
        .open(save_path.join("metadata.csv"))
        .unwrap();

    writeln!(file, "nframe,acq_nframe,timestamp_raw,exposure_time").unwrap();

    // loop over data
    for image in images.iter() {
        // Format other data as a line in a CSV file
        let line = format!(
            "{},{},{},{}",
            image.nframe, image.acq_nframe, image.timestamp_raw, image.exposure_time,
        );
        // Write the line to the file
        writeln!(file, "{}", line).unwrap();
    }

    Ok(())
}


fn video_writer(rx: Receiver<FramesPacket>) -> Result<(), Box<dyn std::error::Error>> {
    while let Ok(packet) = rx.recv() {

        // save video metadata first
        save_video_metadata(&packet.images, &packet.save_path)?;

        // Get the video resolution from the first frame
        let first_frame = packet.images.front().ok_or("No frames provided")?;
        let (width, height) = (first_frame.data.width(), first_frame.data.height());

        // get the save_path from the packet and append .mp4
        let save_path_with_extension = packet.save_path.with_extension("mp4");
        let save_path_str = Arc::new(save_path_with_extension.to_str().ok_or("Failed to convert path to string")?.to_string());

        let mut ffmpeg_command = Command::new("ffmpeg")
            .args(&[
                "-f", "rawvideo",
                "-pixel_format", "gray",
                "-video_size", &format!("{}x{}", width, height),
                "-framerate", "25",
                "-i", "-",
                "-vf", "format=gray",
                "-vcodec", "h264_nvenc",
                "-preset", "p7",
                "-tune", "hq",
                "-rc", "vbr_hq",
                "-qmin", "1",
                "-qmax", "25",
                "-b:v", "5M",
                "-maxrate", "10M",
                "-bufsize", "20M",
                "-profile:v", "high",
                &save_path_str,
            ])
            .stdin(Stdio::piped())
            .spawn()?;

        let stdin = ffmpeg_command.stdin.as_mut().ok_or("Failed to open stdin")?;

        for frame in packet.images {
            stdin.write_all(&frame.data)?;
        }

        ffmpeg_command.wait()?;
    }

    Ok(())
}



pub fn frame_handler(
    receiver: Receiver<(Arc<ImageData>, MessageType)>,
    n_before: usize,
    n_after: usize,
    save_folder: String,
) {
    log::info!("Starting frame handler");

    // create folder to save files, if doesn't exist
    let save_path = Path::new(&save_folder);
    if !save_path.exists() {
        create_dir_all(save_path).unwrap();
    }
    let (frame_packet_sender, frame_packet_receiver) = unbounded::<FramesPacket>();
    let frame_handler_thread = thread::spawn(move || {
        if let Err(e) = video_writer(frame_packet_receiver) {
            log::error!("Error in video writer: {}", e);
        }
    });

    // define frame buffer
    let max_length = n_before + n_after;
    let mut frame_buffer: VecDeque<Arc<ImageData>> = VecDeque::with_capacity(max_length);

    // define control variables
    let mut switch = false;
    let mut counter = n_after;

    // define variable to save incoming data
    let mut trigger_data: KalmanEstimateRow = Default::default();

    // debug stuff
    let mut i_iter = 0;

    loop {
        i_iter += 1;

        if i_iter % 1000 == 0 {
            log::debug!("Backpressure on receiver: {:?}", receiver.len());
        }

        // get data
        let (image_data, incoming) = receiver.recv().unwrap();
        match incoming {
            MessageType::JsonData(kalman_row) => {
                // save kalman row to variable
                trigger_data = kalman_row;
                switch = true;
                log::info!("Received Kalman data: {:?}", trigger_data);
            }
            MessageType::Text(message) => {
                // break if message is kill
                if message == "kill" {
                    log::info!("Received kill message");
                    break;
                }
            }
            MessageType::Empty => {
                // do nothing
            }
            _ => {
                log::warn!("Received unknown message type");
            }
        }

        // pop front if buffer is full, and add to buffer
        if frame_buffer.len() == max_length {
            frame_buffer.pop_front();
        }
        frame_buffer.push_back(image_data);

        // if the switch is defined (meaning, we are recording a video)
        if switch {
            // susbtract counter by 1
            counter -= 1;

            // if counter reaches zero, it means we captured enough frames
            if counter == 0 {
                let time_to_save = Instant::now();
                // write frames to disk
                log::info!("Writing frames to disk");

                // create folder if it doesn't exist
                let video_name = PathBuf::from(format!(
                    "{}/obj_id_{}_frame_{}",
                    save_folder, trigger_data.obj_id, trigger_data.frame
                ));

                // save video to disk along with metadata
                let packet: FramesPacket = FramesPacket {
                    images: frame_buffer.clone(),
                    save_path: video_name,
                };
                frame_packet_sender.send(packet).unwrap();

                log::debug!("Time to save: {:?}", time_to_save.elapsed());

                // and reset counter and switch
                counter = n_after;
                switch = false;
            }
        }
    }
    frame_handler_thread.join().unwrap();
}
