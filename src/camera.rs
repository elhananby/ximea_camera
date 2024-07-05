use super::cli::Args;

/// Dealing with camera parameters
pub fn set_camera_parameters(cam: &mut xiapi::Camera, args: &Args) -> Result<(), i32> {
    // resolution
    set_resolution(cam, args.width, args.height, args.offset_x, args.offset_y)?;

    // exposure
    // let adjusted_exposure = adjust_exposure(args.exposure, &args.fps);
    // cam.set_exposure(args.exposure)?;

    //log::info!("Exposure set to: {}", adjusted_exposure);
    //log::info!("FPS set to: {}", args.fps);

    // data format
    cam.set_image_data_format(xiapi::XI_IMG_FORMAT::XI_MONO8)?;

    // framerate
    cam.set_acq_timing_mode(xiapi::XI_ACQ_TIMING_MODE::XI_ACQ_TIMING_MODE_FRAME_RATE_LIMIT)?;
    cam.set_framerate(args.fps)?;

    cam.set_limit_bandwidth(cam.limit_bandwidth_maximum()?)?;
    let buffer_size = cam.acq_buffer_size()?;
    cam.set_acq_buffer_size(buffer_size * 4)?;
    cam.set_buffers_queue_size(cam.buffers_queue_size_maximum()?)?;

    // Setup AEAG
    unsafe {
        xiapi::xiSetParamInt(
            **cam,
            xiapi::XI_PRM_AEAG.as_ptr() as *const i8,
            xiapi::XI_SWITCH::XI_ON.try_into().unwrap(),
        );
        xiapi::xiSetParamFloat(**cam, xiapi::XI_PRM_EXP_PRIORITY.as_ptr() as *const i8, 1.0);
        xiapi::xiSetParamInt(
            **cam,
            xiapi::XI_PRM_AE_MAX_LIMIT.as_ptr() as *const i8,
            2000,
        );
        xiapi::xiSetParamFloat(**cam, xiapi::XI_PRM_AEAG_LEVEL.as_ptr() as *const i8, 75.0);
    }

    // recent frame
    cam.recent_frame()?;

    Ok(())
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
