use crate::structs::{FramesPacket, ImageData, MessageType, KalmanEstimateRow};
use anyhow::{Context, Result};
use crossbeam::channel::{unbounded, Receiver};
use std::{
    collections::VecDeque,
    fs::{create_dir_all, File},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};
use rayon::prelude::*;

fn save_video_metadata(images: &VecDeque<Arc<ImageData>>, save_path: &Path) -> Result<()> {
    log::info!("Saving metadata to disk");

    let metadata_path = save_path.with_extension("csv");
    let mut file = File::create(&metadata_path)
        .with_context(|| format!("Failed to create metadata file: {:?}", metadata_path))?;

    writeln!(file, "nframe,acq_nframe,timestamp_raw,exposure_time")
        .context("Failed to write header to metadata file")?;

    for image in images {
        writeln!(
            file,
            "{},{},{},{}",
            image.nframe, image.acq_nframe, image.timestamp_raw, image.exposure_time,
        )
        .context("Failed to write image metadata")?;
    }

    Ok(())
}

fn video_writer(rx: Receiver<FramesPacket>) -> Result<()> {
    while let Ok(packet) = rx.recv() {
        if packet.save_path.to_str().unwrap_or("") == "kill" {
            log::info!("Received kill signal in video writer");
            break;
        }

        save_video_metadata(&packet.images, &packet.save_path)
            .context("Failed to save video metadata")?;

        let first_frame = packet.images.front()
            .context("No frames provided in packet")?;
        let (width, height) = (first_frame.width, first_frame.height);
        let save_path = packet.save_path.with_extension("mp4");

        log::info!("Starting ffmpeg command to save video to {:?}", save_path);

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
                save_path.to_str().unwrap(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .context("Failed to start ffmpeg command")?;

        let stdin = ffmpeg_command.stdin.take()
            .context("Failed to open stdin for ffmpeg command")?;
        let stdin = Arc::new(Mutex::new(stdin));

        packet.images.par_iter().try_for_each(|frame| -> Result<()> {
            let mut stdin = stdin.lock().map_err(|e| anyhow::anyhow!("Failed to lock stdin: {:?}", e))?;
            stdin.write_all(&frame.data)
                .context("Failed to write frame data to ffmpeg")
        })?;

        drop(stdin);

        let ffmpeg_status = ffmpeg_command.wait()
            .context("Failed to wait for ffmpeg command")?;
        log::info!("ffmpeg command finished with status: {:?}", ffmpeg_status);
    }

    Ok(())
}

pub fn frame_handler(
    receiver: Receiver<(Arc<ImageData>, MessageType)>,
    n_before: usize,
    n_after: usize,
    save_folder: String,
) -> Result<()> {
    log::info!("Starting frame handler");

    let save_path = Path::new(&save_folder);
    create_dir_all(save_path)
        .with_context(|| format!("Failed to create save directory: {:?}", save_path))?;

    let (frame_packet_sender, frame_packet_receiver) = unbounded::<FramesPacket>();
    let frame_handler_thread = thread::spawn(move || video_writer(frame_packet_receiver));

    let max_length = n_before + n_after;
    let mut frame_buffer: VecDeque<Arc<ImageData>> = VecDeque::with_capacity(max_length);
    let mut switch = false;
    let mut counter = n_after;
    let mut trigger_data: KalmanEstimateRow = Default::default();

    for (i, (image_data, incoming)) in receiver.iter().enumerate() {
        if i % 1000 == 0 {
            log::debug!("Backpressure on receiver: {}", receiver.len());
        }

        match incoming {
            MessageType::JsonData(kalman_row) => {
                trigger_data = kalman_row;
                switch = true;
                log::info!("Received Kalman data: {:?}", trigger_data);
            }
            MessageType::Text(message) if message == "kill" => {
                log::info!("Received kill message");
                frame_packet_sender.send(FramesPacket {
                    images: VecDeque::new(),
                    save_path: PathBuf::from("kill"),
                }).context("Failed to send kill signal to video writer")?;
                break;
            }
            _ => {}
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

                let video_name = save_path.join(format!(
                    "obj_id_{}_frame_{}",
                    trigger_data.obj_id,
                    trigger_data.frame
                ));

                frame_packet_sender.send(FramesPacket {
                    images: frame_buffer.clone(),
                    save_path: video_name,
                }).context("Failed to send frame packet to video writer")?;

                log::debug!("Time to save: {:?}", time_to_save.elapsed());

                counter = n_after;
                switch = false;
            }
        }
    }

    frame_handler_thread.join()
        .map_err(|e| anyhow::anyhow!("Frame handler thread panicked: {:?}", e))?
        .context("Error in video writer thread")?;

    Ok(())
}