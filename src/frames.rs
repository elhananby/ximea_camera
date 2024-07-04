use crate::{
    structs::{FramesPacket, ImageData, MessageType},
    KalmanEstimateRow,
};
use anyhow::{Context, Result};
use crossbeam::channel::{unbounded, Receiver};
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
        let save_path_str = packet
            .save_path
            .with_extension("mp4")
            .to_str()
            .context("Failed to convert path to string")?
            .to_string();

        let mut ffmpeg_command = Command::new("ffmpeg")
            .args([
                "-f",
                "rawvideo",
                "-pixel_format",
                "gray",
                "-video_size",
                &format!("{}x{}", width, height),
                "-framerate",
                "25",
                "-i",
                "-",
                "-vf",
                "format=gray",
                "-vcodec",
                "h264_nvenc",
                "-preset",
                "p7",
                "-tune",
                "hq",
                "-rc",
                "vbr_hq",
                "-qmin",
                "1",
                "-qmax",
                "25",
                "-b:v",
                "5M",
                "-maxrate",
                "10M",
                "-bufsize",
                "20M",
                "-profile:v",
                "high",
                &save_path_str,
            ])
            .stdin(Stdio::piped())
            .spawn()
            .context("Failed to start ffmpeg command")?;

        let stdin = ffmpeg_command
            .stdin
            .as_mut()
            .context("Failed to open stdin")?;

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

        let (image_data, incoming) = receiver.recv().unwrap();
        match incoming {
            MessageType::JsonData(kalman_row) => {
                trigger_data = kalman_row;
                switch = true;
                log::info!("Received Kalman data: {:?}", trigger_data);
            }
            MessageType::Text(message) => {
                if message == "kill" {
                    log::info!("Received kill message");
                    frame_packet_sender
                        .send(FramesPacket {
                            images: VecDeque::new(),
                            save_path: PathBuf::from("kill"),
                        })
                        .unwrap();
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
                frame_packet_sender.send(packet).unwrap();

                log::debug!("Time to save: {:?}", time_to_save.elapsed());

                counter = n_after;
                switch = false;
            }
        }
    }
    frame_handler_thread.join().unwrap();
}



#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::channel::{Sender};
    use image::{ImageBuffer, Luma};
    use rand::Rng;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    fn generate_test_image(nframe: u32) -> Arc<ImageData> {
        let width = 640;
        let height = 480;
        let data = ImageBuffer::from_pixel(width, height, Luma([0u8]));
        Arc::new(ImageData {
            data,
            width,
            height,
            nframe,
            acq_nframe: nframe,
            timestamp_raw: 0,
            exposure_time: 0,
        })
    }

    fn generate_frames(sender: Sender<(Arc<ImageData>, MessageType)>, frame_count: u32) {
        let mut rng = rand::thread_rng();
        for i in 0..frame_count {
            let frame = generate_test_image(i);
            sender.send((frame, MessageType::Empty)).unwrap();
            let jitter = rng.gen_range(10..50);
            thread::sleep(Duration::from_millis(jitter));
        }
    }

    fn generate_trigger(sender: Sender<(Arc<ImageData>, MessageType)>) {
        let trigger_data = KalmanEstimateRow {
            obj_id: 1,
            frame: 100,
            timestamp: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            xvel: 0.0,
            yvel: 0.0,
            zvel: 0.0,
            P00: 0.0,
            P01: 0.0,
            P02: 0.0,
            P11: 0.0,
            P12: 0.0,
            P22: 0.0,
            P33: 0.0,
            P44: 0.0,
            P55: 0.0,
        };

        sender
            .send((generate_test_image(0), MessageType::JsonData(trigger_data)))
            .unwrap();
    }

    #[test]
    fn test_frame_handler() {
        let (frame_sender, frame_receiver) = unbounded();
        let save_folder = String::from("test_output");
        let n_before = 5;
        let n_after = 5;

        // Start the frame handler in a separate thread
        let frame_handler_thread = thread::spawn(move || {
            frame_handler(frame_receiver, n_before, n_after, save_folder);
        });

        // Generate frames in a separate thread
        let frame_sender_clone = frame_sender.clone();
        let frame_generation_thread = thread::spawn(move || {
            generate_frames(frame_sender_clone, 100);
        });

        // Introduce random jitter before sending the trigger signal
        let frame_sender_clone = frame_sender.clone();
        let trigger_thread = thread::spawn(move || {
            thread::sleep(Duration::from_secs(2));
            generate_trigger(frame_sender_clone);
        });

        // Wait for the threads to complete their execution
        frame_generation_thread.join().unwrap();
        trigger_thread.join().unwrap();

        // Send kill signal to stop the frame handler
        frame_sender.send((generate_test_image(0), MessageType::Text(String::from("kill")))).unwrap();

        // Wait for the frame handler to complete its execution
        frame_handler_thread.join().unwrap();

        // Validate the output (this part would need to be implemented based on your specific requirements)
        // For example, check if the output file exists and has the expected size, etc.
        assert!(Path::new("test_output/obj_id_1_frame_100.mp4").exists());
    }
}