use anyhow::{Context, Result};
use log::info;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::frame::Frame;

pub fn save_video_metadata(frames: &[Frame], save_path: &Path) -> Result<()> {
    info!("Saving video metadata to {}", save_path.display());

    let mut file = File::create(save_path)
        .with_context(|| format!("Failed to create metadata file at {}", save_path.display()))?;

    writeln!(file, "nframe,acq_nframe,timestamp_raw,exposure_time")
        .context("Failed to write header to metadata file")?;

    for frame in frames {
        writeln!(
            file,
            "{},{},{},{}",
            frame.nframe, frame.acq_nframe, frame.timestamp_raw, frame.exposure_time
        )
        .context("Failed to write frame metadata")?;
    }

    info!("Successfully saved metadata for {} frames", frames.len());

    Ok(())
}