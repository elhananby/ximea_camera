use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ximea_camera::video::{FrameHandler, FfmpegWriter};
use ximea_camera::camera::ImageData;
use ximea_camera::messaging::Message;
use std::sync::Arc;
use std::path::PathBuf;
use image::{ImageBuffer, Luma};

fn create_dummy_image(width: u32, height: u32) -> ImageData {
    ImageData {
        data: ImageBuffer::<Luma<u8>, Vec<u8>>::new(width, height),
        width,
        height,
        nframe: 0,
        acq_nframe: 0,
        timestamp_raw: 0,
        exposure_time: 0,
    }
}

fn benchmark_frame_handling(c: &mut Criterion) {
    let temp_dir = tempfile::tempdir().unwrap();
    let video_writer = Box::new(FfmpegWriter::new(temp_dir.path().to_path_buf()).unwrap());
    let mut frame_handler = FrameHandler::new(
        10,  // n_before
        20,  // n_after
        video_writer,
        temp_dir.path().to_path_buf(),
    );

    let dummy_image = Arc::new(create_dummy_image(640, 480));
    let dummy_message = Some(Message::JsonData("{}".to_string()));  // Adjust as necessary

    c.bench_function("handle single frame", |b| {
        b.iter(|| {
            frame_handler.handle_frame(black_box(dummy_image.clone()), black_box(dummy_message.clone())).unwrap();
        })
    });

    c.bench_function("handle 100 frames", |b| {
        b.iter(|| {
            for _ in 0..100 {
                frame_handler.handle_frame(black_box(dummy_image.clone()), black_box(None)).unwrap();
            }
        })
    });
}

criterion_group!(benches, benchmark_frame_handling);
criterion_main!(benches);