// Standard library imports
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// Current crate and supermodule imports
use super::structs::{Args, MessageType};
use crate::KalmanEstimateRow;

#[allow(dead_code)]
pub fn time() -> f64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let seconds = duration.as_secs() as f64; // Convert seconds to f64
            let nanos = duration.subsec_nanos() as f64; // Convert nanoseconds to f64
            seconds + nanos / 1_000_000_000.0 // Combine both into a single f64 value
        }
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}

/// Dealing with camera parameters
pub fn set_camera_parameters(cam: &mut xiapi::Camera, args: &Args) -> Result<(), i32> {
    // lens mode
    //set_lens_mode_with_retry(cam, args.aperture)?;

    // resolution
    set_resolution(cam, args.width, args.height, args.offset_x, args.offset_y)?;

    // exposure
    let adjusted_exposure = adjust_exposure(args.exposure, &args.fps);
    cam.set_exposure(args.exposure)?;

    log::info!("Exposure set to: {}", adjusted_exposure);
    log::info!("FPS set to: {}", args.fps);

    // data format
    cam.set_image_data_format(xiapi::XI_IMG_FORMAT::XI_MONO8)?;

    // framerate
    cam.set_acq_timing_mode(xiapi::XI_ACQ_TIMING_MODE::XI_ACQ_TIMING_MODE_FRAME_RATE_LIMIT)?;
    cam.set_framerate(args.fps)?;

    cam.set_limit_bandwidth(cam.limit_bandwidth_maximum()?)?;
    let buffer_size = cam.acq_buffer_size()?;
    cam.set_acq_buffer_size(buffer_size * 4)?;
    cam.set_buffers_queue_size(cam.buffers_queue_size_maximum()?)?;

    // Flat-field correction
    // TODO: Implement flat-field correction
    // unsafe {
    //     let image = String::from("/home/buchsbaum/Documents/dark_image.tif");
    //     let c_path = CString::new(image.clone()).expect("Failed to convert to CString");
    //     let raw_ptr = c_path.as_ptr();
    //     let c_void_ptr: *mut c_void = raw_ptr as *mut c_void;

    //     xiapi::xiSetParamString(
    //         **cam,
    //         xiapi::XI_PRM_FFC_DARK_FIELD_FILE_NAME.as_ptr() as *const i8,
    //         c_void_ptr,
    //         image.len() as u32,
    //     );

    //     let image = String::from("/home/buchsbaum/Documents/mid_image.tif");
    //     let c_path = CString::new(image.clone()).expect("Failed to convert to CString");
    //     let raw_ptr = c_path.as_ptr();
    //     let c_void_ptr: *mut c_void = raw_ptr as *mut c_void;

    //     xiapi::xiSetParamString(
    //         **cam,
    //         xiapi::XI_PRM_FFC_FLAT_FIELD_FILE_NAME.as_ptr() as *const i8,
    //         c_void_ptr,
    //         image.len() as u32,
    //     );

    //     xiapi::xiSetParamInt(**cam, xiapi::XI_PRM_FFC.as_ptr() as *const i8, 1);
    // }

    // recent frame
    cam.recent_frame()?;

    Ok(())
}

fn get_offset_for_resolution(
    max_resolution: (u32, u32),
    width: u32,
    height: u32,
) -> Result<(u32, u32), i32> {
    let mut offset_x = (max_resolution.0 - width) / 2;
    let mut offset_y = (max_resolution.1 - height) / 2;

    offset_x = ((offset_x as f32 / 32.0).ceil() * 32_f32) as u32;
    offset_y = ((offset_y as f32 / 32.0).ceil() * 32_f32) as u32;
    log::debug!("Offset x = {}, Offset y = {}", offset_x, offset_y);
    Ok((offset_x, offset_y))
}

fn adjust_exposure(exposure: f32, fps: &f32) -> f32 {
    let max_exposure_for_fps = 1_000_000_f32 / fps;

    // if the exposure is greater than the max exposure for the fps
    // return the max exposure (-1.0 to make sure it's short enough) possible for the fps
    // otherwise return the original exposure
    if exposure > max_exposure_for_fps {
        max_exposure_for_fps - 1.0
    } else {
        exposure
    }
}

fn set_lens_mode_with_retry(cam: &mut xiapi::Camera, aperture: f32) -> Result<(), i32> {
    let mut max_retries = 5;
    let delay = std::time::Duration::from_secs(2);

    cam.set_lens_mode(xiapi::XI_SWITCH::XI_ON)?;
    log::info!("Lens mode set to XI_ON");

    loop {
        // match
        match cam.set_lens_aperture_value(aperture) {
            Ok(_) => {
                log::info!("Lens aperture value set to 5.2");
                return Ok(()); // return Ok if the aperture is set successfully
            }
            Err(e) => {
                log::debug!("Error setting lens aperture value: {}, retrying.", e);

                max_retries -= 1;
                if max_retries == 0 {
                    log::error!("Max retries reached, exiting.");
                    return Err(e); // return Err if the max retries are reached
                }

                std::thread::sleep(delay);
                continue;
            }
        }
    }
}

fn set_resolution(
    cam: &mut xiapi::Camera,
    width: u32,
    height: u32,
    offset_x: u32,
    offset_y: u32,
) -> Result<(), i32> {
    let _max_resolution = cam.roi().unwrap();

    //let (offset_x, offset_y) = get_offset_for_resolution((max_resolution.width, max_resolution.height), width, height)?;

    let roi = xiapi::Roi {
        offset_x,
        offset_y,
        width,
        height,
    };
    let actual_roi = cam.set_roi(&roi);

    log::debug!(
        "Current resolution = {:?}x{:?}",
        actual_roi.as_ref().unwrap().width,
        actual_roi.as_ref().unwrap().height
    );

    Ok(())
}

/// ZMQ handling

pub fn connect_to_socket(port: &str, socket_type: zmq::SocketType) -> zmq::Socket {
    let context = zmq::Context::new();
    let socket = context.socket(socket_type).unwrap();
    socket
        .connect(format!("tcp://127.0.0.1:{}", port).as_str())
        .unwrap();
    if socket_type == zmq::SUB {
        socket.set_subscribe(b"trigger").unwrap();
    };
    socket
}
pub fn parse_message(message: &str) -> MessageType {
    if message.trim().is_empty() {
        return MessageType::Empty;
    }

    match serde_json::from_str::<KalmanEstimateRow>(message) {
        Ok(data) => MessageType::JsonData(data),
        Err(e) => {
            if e.is_data() {
                // If the error is due to data format issues, return InvalidJson
                MessageType::InvalidJson(message.to_string(), e)
            } else {
                // For other types of errors, treat it as a plain text message
                MessageType::Text(message.to_string())
            }
        }
    }
}

/// Ctrl-C handling
pub fn setup_ctrlc_handler(running: Arc<AtomicBool>) {
    ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
}
