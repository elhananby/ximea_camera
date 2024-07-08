use anyhow::{Context, Result};
use tokio::task;
use crate::structs::Args;

#[derive(thiserror::Error, Debug)]
pub enum CameraError {
    #[error("Failed to set camera parameter: {0}")]
    ParameterError(String),
    #[error("Camera operation failed: {0}")]
    OperationError(String),
}

/// Dealing with camera parameters
pub async fn set_camera_parameters(cam: &mut xiapi::Camera, args: &Args) -> Result<()> {
    // resolution
    set_resolution(cam, args.width, args.height, args.offset_x, args.offset_y).await?;

    // data format
    cam.set_image_data_format(xiapi::XI_IMG_FORMAT::XI_MONO8)
        .context("Failed to set image data format")?;

    // framerate
    cam.set_acq_timing_mode(xiapi::XI_ACQ_TIMING_MODE::XI_ACQ_TIMING_MODE_FRAME_RATE_LIMIT)
        .context("Failed to set acquisition timing mode")?;
    cam.set_framerate(args.fps)
        .context("Failed to set framerate")?;

    cam.set_limit_bandwidth(cam.limit_bandwidth_maximum()?)
        .context("Failed to set bandwidth limit")?;
    let buffer_size = cam.acq_buffer_size()?;
    cam.set_acq_buffer_size(buffer_size * 4)
        .context("Failed to set acquisition buffer size")?;
    cam.set_buffers_queue_size(cam.buffers_queue_size_maximum()?)
        .context("Failed to set buffers queue size")?;

    // Setup AEAG
    setup_aeag(cam).await?;

    // recent frame
    cam.recent_frame()
        .context("Failed to set recent frame")?;

    Ok(())
}

async fn setup_aeag(cam: &mut xiapi::Camera) -> Result<()> {
    task::spawn_blocking(move || -> Result<()> {
        unsafe {
            xiapi::xiSetParamInt(
                **cam,
                xiapi::XI_PRM_AEAG.as_ptr() as *const i8,
                xiapi::XI_SWITCH::XI_ON.try_into().unwrap(),
            ).map_err(|e| CameraError::ParameterError(format!("Failed to set AEAG: {}", e)))?;
            xiapi::xiSetParamFloat(**cam, xiapi::XI_PRM_EXP_PRIORITY.as_ptr() as *const i8, 1.0)
                .map_err(|e| CameraError::ParameterError(format!("Failed to set exposure priority: {}", e)))?;
            xiapi::xiSetParamInt(
                **cam,
                xiapi::XI_PRM_AE_MAX_LIMIT.as_ptr() as *const i8,
                2000,
            ).map_err(|e| CameraError::ParameterError(format!("Failed to set AE max limit: {}", e)))?;
            xiapi::xiSetParamFloat(**cam, xiapi::XI_PRM_AEAG_LEVEL.as_ptr() as *const i8, 75.0)
                .map_err(|e| CameraError::ParameterError(format!("Failed to set AEAG level: {}", e)))?;
        }
        Ok(())
    }).await?
}

async fn set_resolution(
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
        .context("Failed to set ROI")?;

    tracing::debug!(
        "Current resolution = {:?}x{:?}",
        actual_roi.width,
        actual_roi.height
    );

    Ok(())
}

#[allow(dead_code)]
async fn get_offset_for_resolution(
    max_resolution: (u32, u32),
    width: u32,
    height: u32,
) -> Result<(u32, u32)> {
    let offset_x = (max_resolution.0 - width) / 2;
    let offset_y = (max_resolution.1 - height) / 2;

    let offset_x = ((offset_x as f32 / 32.0).ceil() * 32_f32) as u32;
    let offset_y = ((offset_y as f32 / 32.0).ceil() * 32_f32) as u32;
    tracing::debug!("Offset x = {}, Offset y = {}", offset_x, offset_y);
    Ok((offset_x, offset_y))
}

#[allow(dead_code)]
fn adjust_exposure(exposure: f32, fps: &f32) -> f32 {
    let max_exposure_for_fps = 1_000_000_f32 / fps;
    (exposure > max_exposure_for_fps).then(|| max_exposure_for_fps - 1.0).unwrap_or(exposure)
}