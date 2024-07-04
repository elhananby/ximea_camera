use std::{
    collections::VecDeque,
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::Instant,
};

use crossbeam::channel::{Receiver, unbounded};

use crate::{
    structs::{ImageData, MessageType, FramesPacket},
    KalmanEstimateRow,
};

fn save_video_metadata(
    images: &VecDeque<Arc<ImageData>>,
    save_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    log::debug!("Saving metadata to disk");

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(save_path.join("metadata.csv"))?;

    writeln!(file, "nframe,acq_nframe,timestamp_raw,exposure_time")?;

    for image in images.iter() {
        let line = format!(
            "{},{},{},{}",
            image.nframe, image.acq_nframe, image.timestamp_raw, image.exposure_time,
        );
        writeln!(file, "{}", line)?;
    }

    Ok(())
}

fn video_writer(rx: Receiver<FramesPacket>) -> Result<(), Box<dyn std::error::Error>> {
    while let Ok(packet) = rx.recv() {
        if packet.save_path.to_str().unwrap_or("") == "kill" {
            log::info!("Received kill signal in video writer");
            break;
        }

        save_video_metadata(&packet.images, &packet.save_path)?;

        let first_frame = packet.images.front().ok_or("No frames provided")?;
        let (width, height) = (first_frame.width, first_frame.height);

        let save_path_with_extension = packet.save_path.with_extension("mp4");
        let save_path_str = Arc::new(save_path_with_extension.to_str().ok_or("Failed to convert path to string")?.to_string());

        let mut ffmpeg_command = Command::new("ffmpeg")
            .args([
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

    let save_path = Path::new(&save_folder);
    if !save_path.exists() {
        if let Err(e) = create_dir_all(save_path) {
            log::error!("Failed to create save directory: {}", e);
            return;
        }
    }

    let (frame_packet_sender, frame_packet_receiver) = unbounded::<FramesPacket>();
    let frame_handler_thread = thread::spawn(move || {
        if let Err(e) = video_writer(frame_packet_receiver) {
            log::error!("Error in video writer: {}", e);
        }
    });

    let max_length = n_before + n_after;
    let mut frame_buffer: VecDeque<Arc<ImageData>> = VecDeque::with_capacity(max_length);

    let mut switch = false;
    let mut counter = n_after;

    let mut trigger_data: KalmanEstimateRow = Default::default();

    let mut i_iter = 0;

    loop {
        i_iter += 1;

        if i_iter % 1000 == 0 {
            log::debug!("Backpressure on receiver: {:?}", receiver.len());
        }

        let (image_data, incoming) = match receiver.recv() {
            Ok(data) => data,
            Err(_) => {
                log::info!("Receiver channel closed");
                break;
            }
        };

        match incoming {
            MessageType::JsonData(kalman_row) => {
                trigger_data = kalman_row;
                switch = true;
                log::info!("Received Kalman data: {:?}", trigger_data);
            }
            MessageType::Text(message) => {
                if message == "kill" {
                    log::info!("Received kill message");
                    frame_packet_sender.send(FramesPacket {
                        images: VecDeque::new(),
                        save_path: PathBuf::from("kill"),
                    }).unwrap();
                    break;
                }
            }
            MessageType::Empty => (),
            _ => {
                log::warn!("Received unknown message type");
            }
        }

        if frame_buffer.len() == max_length {
            frame_buffer.pop_front();
        }
        frame_buffer.push_back(image_data);

        if switch {
            counter -= 1;

            if counter == 0 {
                let time_to_save = Instant::now();
                log::info!("Writing frames to disk");

                let video_name = PathBuf::from(format!(
                    "{}/obj_id_{}_frame_{}",
                    save_folder, trigger_data.obj_id, trigger_data.frame
                ));

                let packet: FramesPacket = FramesPacket {
                    images: frame_buffer.clone(),
                    save_path: video_name,
                };
                frame_packet_sender.send(packet).unwrap();

                log::debug!("Time to save: {:?}", time_to_save.elapsed());

                counter = n_after;
                switch = false;
            }
        }
    }

    if let Err(e) = frame_handler_thread.join() {
        log::error!("Failed to join frame handler thread: {:?}", e);
    }
}
