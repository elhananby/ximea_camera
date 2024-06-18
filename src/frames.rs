// Standard library imports
use std::{
    collections::VecDeque,
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

// External crates
use crossbeam::channel::Receiver;
use image::ImageFormat;
use rayon::prelude::*;

// Current crate
use crate::{
    structs::{ImageData, MessageType},
    KalmanEstimateRow,
};


extern crate ffmpeg_next as ffmpeg;
use ffmpeg::{
    codec, encoder, format,
};

const DEFAULT_X264_OPTS: &str = "preset=medium";

fn save_images_to_disk(
    images: &VecDeque<Arc<ImageData>>,
    save_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    log::debug!("Saving images to disk");

    // loop over images and save to disk
    images.into_par_iter().for_each(|image| {
        // set the filename to save the image to (based on the acq_nframe field of the image)
        let filename = save_path.join(format!("{}.tiff", image.acq_nframe));

        // save the image to disk
        match image.data.save_with_format(&filename, ImageFormat::Tiff) {
            // print a debug message if the image was saved successfully
            Ok(_) => {}
            // print an error message if the image failed to save
            Err(e) => log::debug!("Failed to save {}: {}", filename.display(), e),
        }
    });

    Ok(())
}

fn save_video_to_disk(_images: &VecDeque<Arc<ImageData>>, save_path: &Path) {
    log::info!("Saving video to disk");

    let output_file = save_path.join("video.mp4");
    // Initialize ffmpeg library
    ffmpeg_next::init().unwrap();

    let mut octx = format::output(&output_file).unwrap();
    let _codec = encoder::find(codec::Id::H264);
    let _x264_opts = DEFAULT_X264_OPTS;

    octx.write_header().unwrap();
}

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
                log::debug!("Received Kalman data: {:?}", trigger_data);
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
                let save_folder = format!(
                    "{}/obj_id_{}_frame_{}",
                    save_folder, trigger_data.obj_id, trigger_data.frame
                );
                let save_folder = PathBuf::from(save_folder);

                if !Path::new(&save_folder).exists() {
                    create_dir_all(&save_folder).unwrap();
                }

                // save images to disk using parallel execution
                save_images_to_disk(&frame_buffer, &save_folder).unwrap();
                save_video_metadata(&frame_buffer, &save_folder).unwrap();
                log::debug!("Time to save: {:?}", time_to_save.elapsed());

                // and reset counter and switch
                counter = n_after;
                switch = false;
            }
        }
    }
}
