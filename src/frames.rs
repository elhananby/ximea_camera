// Standard library imports
use std::{
    collections::VecDeque, fs::{create_dir_all, OpenOptions}, io::Write, path::{Path, PathBuf}, process::{Command, Stdio}, sync::Arc, thread, time::Instant
};
use crossbeam::channel::{Receiver, unbounded};
use anyhow::{Result, Context};
use crate::{
    structs::{ImageData, MessageType, FramesPacket},
    KalmanEstimateRow,
};

fn save_video_metadata(images: &VecDeque<Arc<ImageData>>, save_path: &Path) -> Result<()> {
    log::debug!("Saving metadata to disk");

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(save_path.join("metadata.csv"))
        .context("Failed to open metadata file")?;

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

fn video_writer(rx: Receiver<FramesPacket>) -> Result<()> {
    while let Ok(packet) = rx.recv() {
        if packet.save_path.to_str().unwrap_or("") == "kill" {
            log::info!("Received kill signal in video writer");
            break;
        }

        save_video_metadata(&packet.images, &packet.save_path)?;

        let first_frame = packet.images.front().context("No frames provided")?;
        let (width, height) = (first_frame.width, first_frame.height);
        let save_path_str = packet.save_path.with_extension("mp4")
            .to_str()
            .context("Failed to convert path to string")?
            .to_string();

        log::info!("Starting ffmpeg command to save video to {}", save_path_str);

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
            .spawn()
            .context("Failed to start ffmpeg command")?;

        let stdin = ffmpeg_command.stdin.as_mut().context("Failed to open stdin")?;

        for frame in packet.images {
            stdin.write_all(&frame.data)?;
        }

        let ffmpeg_status = ffmpeg_command.wait()?;
        log::info!("ffmpeg command finished with status: {:?}", ffmpeg_status);
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
        create_dir_all(save_path).unwrap();
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
                log::error!("Failed to receive data");
                break;
            }
        };

        match incoming {
            MessageType::JsonData(kalman_row) => {
                trigger_data = kalman_row;
                switch = true;
                log::info!("Received Kalman data");
                log::debug!("{:?}", trigger_data);
            }
            MessageType::Text(message) => {
                if message == "kill" {
                    log::info!("Received kill message");
                    if frame_packet_sender.send(FramesPacket {
                        images: VecDeque::new(),
                        save_path: PathBuf::from("kill"),
                    }).is_err() {
                        log::error!("Failed to send kill signal");
                    }
                    break;
                }
            }
            MessageType::Empty => {}
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

                let packet = FramesPacket {
                    images: frame_buffer.clone(),
                    save_path: video_name,
                };
                if frame_packet_sender.send(packet).is_err() {
                    log::error!("Failed to send frame packet");
                }

                log::debug!("Time to save: {:?}", time_to_save.elapsed());

                counter = n_after;
                switch = false;
            }
        }
    }
    frame_handler_thread.join().unwrap();
}