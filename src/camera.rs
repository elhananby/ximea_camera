use super::structs::Args;
use anyhow::{Context, Result, anyhow};

/// Set camera parameters based on provided arguments
pub fn set_camera_parameters(cam: &mut xiapi::Camera, args: &Args) -> Result<()> {
    // Set resolution
    set_resolution(cam, args.width, args.height, args.offset_x, args.offset_y)
        .context("Failed to set resolution")?;

    // Set image data format
    cam.set_image_data_format(xiapi::XI_IMG_FORMAT::XI_MONO8)
        .map_err(|e| anyhow!("Failed to set image data format: {}", e))
        .context("Camera error")?;

    // Set framerate
    cam.set_acq_timing_mode(xiapi::XI_ACQ_TIMING_MODE::XI_ACQ_TIMING_MODE_FRAME_RATE_LIMIT)
        .map_err(|e| anyhow!("Failed to set acquisition timing mode: {}", e))
        .context("Camera error")?;
    cam.set_framerate(args.fps)
        .map_err(|e| anyhow!("Failed to set framerate: {}", e))
        .context("Camera error")?;

    // Optimize buffer settings
    optimize_buffer_settings(cam)?;

    // Setup AEAG (Auto Exposure Auto Gain)
    setup_aeag(cam)?;

    // Enable recent frame
    cam.recent_frame()
        .map_err(|e| anyhow!("Failed to enable recent frame: {}", e))
        .context("Camera error")?;

    Ok(())
}

fn optimize_buffer_settings(cam: &mut xiapi::Camera) -> Result<()> {
    let max_bandwidth = cam.limit_bandwidth_maximum()
        .map_err(|e| anyhow!("Failed to get maximum bandwidth: {}", e))
        .context("Camera error")?;
    cam.set_limit_bandwidth(max_bandwidth)
        .map_err(|e| anyhow!("Failed to set bandwidth limit: {}", e))
        .context("Camera error")?;

    let buffer_size = cam.acq_buffer_size()
        .map_err(|e| anyhow!("Failed to get buffer size: {}", e))
        .context("Camera error")?;
    cam.set_acq_buffer_size(buffer_size * 4)
        .map_err(|e| anyhow!("Failed to set acquisition buffer size: {}", e))
        .context("Camera error")?;

    let max_queue_size = cam.buffers_queue_size_maximum()
        .map_err(|e| anyhow!("Failed to get maximum queue size: {}", e))
        .context("Camera error")?;
    cam.set_buffers_queue_size(max_queue_size)
        .map_err(|e| anyhow!("Failed to set buffers queue size: {}", e))
        .context("Camera error")?;

    Ok(())
}

fn setup_aeag(cam: &mut xiapi::Camera) -> Result<()> {
    unsafe {
        let result = xiapi::xiSetParamInt(
            **cam,
            xiapi::XI_PRM_AEAG.as_ptr() as *const i8,
            xiapi::XI_SWITCH::XI_ON.try_into().unwrap(),
        );
        if result != 0 {
            return Err(anyhow!("Failed to enable AEAG: {}", result)).context("Camera error");
        }

        let result = xiapi::xiSetParamFloat(**cam, xiapi::XI_PRM_EXP_PRIORITY.as_ptr() as *const i8, 1.0);
        if result != 0 {
            return Err(anyhow!("Failed to set exposure priority: {}", result)).context("Camera error");
        }

        let result = xiapi::xiSetParamInt(**cam, xiapi::XI_PRM_AE_MAX_LIMIT.as_ptr() as *const i8, 2000);
        if result != 0 {
            return Err(anyhow!("Failed to set AE max limit: {}", result)).context("Camera error");
        }

        let result = xiapi::xiSetParamFloat(**cam, xiapi::XI_PRM_AEAG_LEVEL.as_ptr() as *const i8, 75.0);
        if result != 0 {
            return Err(anyhow!("Failed to set AEAG level: {}", result)).context("Camera error");
        }
    }

    Ok(())
}

fn set_resolution(
    cam: &mut xiapi::Camera,
    width: u32,
    height: u32,
    offset_x: u32,
    offset_y: u32,
) -> Result<()> {
    let roi = xiapi::Roi {
        offset_x,
        offset_y,
        width,
        height,
    };
    let actual_roi = cam.set_roi(&roi)
        .map_err(|e| anyhow!("Failed to set ROI: {}", e))
        .context("Camera error")?;

    log::debug!(
        "Current resolution = {}x{}",
        actual_roi.width,
        actual_roi.height
    );

    Ok(())
}

#[allow(dead_code)]
fn get_offset_for_resolution(
    max_resolution: (u32, u32),
    width: u32,
    height: u32,
) -> Result<(u32, u32)> {
    let offset_x = ((max_resolution.0 - width) / 2 + 31) / 32 * 32;
    let offset_y = ((max_resolution.1 - height) / 2 + 31) / 32 * 32;
    log::debug!("Offset x = {}, Offset y = {}", offset_x, offset_y);
    Ok((offset_x, offset_y))
}

#[allow(dead_code)]
fn adjust_exposure(exposure: f32, fps: f32) -> f32 {
    let max_exposure_for_fps = 1_000_000.0 / fps;
    exposure.min(max_exposure_for_fps - 1.0)
}