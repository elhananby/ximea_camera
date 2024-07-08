use crate::{
    structs::{FramesPacket, ImageData, MessageType},
    KalmanEstimateRow,
};
use anyhow::{Context, Result};
use tokio::sync::mpsc::{self, Receiver, Sender};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Instant,
};
use tokio::{fs::File, io::AsyncWriteExt, task};

#[derive(thiserror::Error, Debug)]
enum FrameError {
    #[error("Failed to save metadata: {0}")]
    MetadataSaveError(#[from] std::io::Error),
    #[error("Failed to start ffmpeg: {0}")]
    FfmpegStartError(std::io::Error),
    #[error("Failed to write frame to ffmpeg: {0}")]
    FfmpegWriteError(std::io::Error),
}

async fn save_video_metadata(images: &VecDeque<Arc<ImageData>>, save_path: &Path) -> Result<()> {
    tracing::info!("Saving metadata to disk");

    let mut save_path_str = save_path.to_string_lossy().to_string();
    save_path_str.push_str(".csv");

    let new_path: &Path = Path::new(&save_path_str);

    let mut file = File::create(new_path).await.context("Failed to create metadata file")?;

    file.write_all(b"nframe,acq_nframe,timestamp_raw,exposure_time\n").await?;

    for image in images.iter() {
        let line = format!(
            "{},{},{},{}\n",
            image.nframe, image.acq_nframe, image.timestamp_raw, image.exposure_time,
        );
        file.write_all(line.as_bytes()).await?;
    }

    Ok(())
}

async fn video_writer(mut rx: Receiver<FramesPacket>) -> Result<()> {
    while let Some(packet) = rx.recv().await {
        if packet.save_path.to_str().unwrap_or("") == "kill" {
            tracing::info!("Received kill signal in video writer");
            break;
        }
        save_video_metadata(&packet.images, &packet.save_path).await?;

        let first_frame = packet.images.front().context("No frames provided")?;
        let (width, height) = (first_frame.width, first_frame.height);
        let save_path_str = packet
            .save_path
            .with_extension("mp4")
            .to_str()
            .context("Failed to convert path to string")?
            .to_string();

        tracing::info!("Starting ffmpeg command to save video to {}", save_path_str);

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
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .map_err(FrameError::FfmpegStartError)?;

        let stdin = ffmpeg_command.stdin.as_mut().context("Failed to open stdin")?;

        tracing::info!("Writing frames to ffmpeg");
        for frame in packet.images {
            stdin.write_all(&frame.data).map_err(FrameError::FfmpegWriteError)?;
        }

        let ffmpeg_status = ffmpeg_command.wait()?;
        tracing::info!("ffmpeg command finished with status: {:?}", ffmpeg_status);
    }

    Ok(())
}

pub async fn frame_handler(
    mut receiver: Receiver<(Arc<ImageData>, MessageType)>,
    n_before: usize,
    n_after: usize,
    save_folder: String,
) -> Result<()> {
    tracing::info!("Starting frame handler");

    let save_path = Path::new(&save_folder);
    if !save_path.exists() {
        tokio::fs::create_dir_all(save_path).await.context("Failed to create save directory")?;
    }
    let (frame_packet_sender, frame_packet_receiver) = mpsc::channel::<FramesPacket>(100);
    let frame_handler_thread = tokio::spawn(async move {
        if let Err(e) = video_writer(frame_packet_receiver).await {
            tracing::error!("Error in video writer: {}", e);
        }
    });

    let max_length = n_before + n_after;
    let mut frame_buffer: VecDeque<Arc<ImageData>> = VecDeque::with_capacity(max_length);
    let mut switch = false;
    let mut counter = n_after;
    let mut trigger_data: KalmanEstimateRow = Default::default();
    let mut i_iter = 0;

    while let Some((image_data, incoming)) = receiver.recv().await {
        i_iter += 1;

        if i_iter % 1000 == 0 {
            tracing::debug!("Backpressure on receiver: {:?}", receiver.capacity());
        }

        match incoming {
            MessageType::JsonData(kalman_row) => {
                trigger_data = kalman_row;
                switch = true;
                tracing::info!("Received Kalman data");
                tracing::debug!("{:?}", trigger_data);
            }
            MessageType::Text(message) => {
                if message == "kill" {
                    tracing::info!("Received kill message");
                    if frame_packet_sender
                        .send(FramesPacket {
                            images: VecDeque::new(),
                            save_path: PathBuf::from("kill"),
                        })
                        .await
                        .is_err()
                    {
                        tracing::error!("Failed to send kill signal");
                    }
                    break;
                }
            }
            MessageType::Empty => {}
            MessageType::InvalidJson(_, _) => {
                tracing::warn!("Received invalid JSON message");
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
                tracing::info!("Writing frames to disk");

                let video_name = PathBuf::from(format!(
                    "{}/obj_id_{}_frame_{}",
                    save_folder, trigger_data.obj_id, trigger_data.frame
                ));

                let packet = FramesPacket {
                    images: frame_buffer.clone(),
                    save_path: video_name,
                };
                if frame_packet_sender.send(packet).await.is_err() {
                    tracing::error!("Failed to send frame packet");
                }

                tracing::debug!("Time to save: {:?}", time_to_save.elapsed());

                counter = n_after;
                switch = false;
            }
        }
    }
    frame_handler_thread.await?;
    Ok(())
}