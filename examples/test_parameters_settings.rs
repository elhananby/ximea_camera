fn main() -> Result<(), i32> {
    let mut cam = xiapi::open_device(None)?;
    // data format
    cam.set_image_data_format(xiapi::XI_IMG_FORMAT::XI_MONO8)?;

    // framerate
    cam.set_acq_timing_mode(xiapi::XI_ACQ_TIMING_MODE::XI_ACQ_TIMING_MODE_FRAME_RATE_LIMIT)?;
    cam.set_framerate(500.0)?;

    cam.set_limit_bandwidth(cam.limit_bandwidth_maximum()?)?;
    let buffer_size = cam.acq_buffer_size()?;
    cam.set_acq_buffer_size(buffer_size * 4)?;
    cam.set_buffers_queue_size(cam.buffers_queue_size_maximum()?)?;

    // Setup AEAG
    unsafe {
        xiapi::xiSetParamInt(
            *cam,
            xiapi::XI_PRM_AEAG.as_ptr() as *const i8,
            xiapi::XI_SWITCH::XI_ON.try_into().unwrap(),
        );
        xiapi::xiSetParamFloat(*cam, xiapi::XI_PRM_EXP_PRIORITY.as_ptr() as *const i8, 1.0);
        xiapi::xiSetParamInt(*cam, xiapi::XI_PRM_AE_MAX_LIMIT.as_ptr() as *const i8, 2000);
        xiapi::xiSetParamFloat(*cam, xiapi::XI_PRM_AEAG_LEVEL.as_ptr() as *const i8, 75.0);
    }

    // recent frame
    cam.recent_frame()?;

    Ok(())
}
