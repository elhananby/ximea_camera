use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::Error;

/// Returns the current time as a f64 representing seconds since the Unix epoch.
pub fn current_time() -> Result<f64, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| {
            let seconds = duration.as_secs() as f64;
            let nanos = duration.subsec_nanos() as f64;
            seconds + nanos / 1_000_000_000.0
        })
        .map_err(|e| Error::Unknown(format!("SystemTime before UNIX EPOCH: {}", e)))
}

/// Calculates the maximum exposure time for a given frame rate.
pub fn calculate_max_exposure(fps: f32) -> f32 {
    (1.0 / fps as f32) * 1_000_000.0 - 1.0 // Convert to microseconds and subtract 1 for safety
}

/// Adjusts the exposure time to ensure it doesn't exceed the maximum allowed for the given frame rate.
pub fn adjust_exposure(exposure: f32, fps: f32) -> f32 {
    let max_exposure = calculate_max_exposure(fps);
    exposure.min(max_exposure)
}

/// Calculates the offset for a given resolution to center it within the maximum resolution.
pub fn calculate_resolution_offset(max_resolution: (u32, u32), width: u32, height: u32) -> (u32, u32) {
    let offset_x = ((max_resolution.0 - width) / 2 + 31) / 32 * 32;
    let offset_y = ((max_resolution.1 - height) / 2 + 31) / 32 * 32;
    (offset_x, offset_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_time() {
        let result = current_time();
        assert!(result.is_ok());
        let time = result.unwrap();
        assert!(time > 0.0);
    }

    #[test]
    fn test_calculate_max_exposure() {
        assert_eq!(calculate_max_exposure(30.0), 32999.0);
        assert_eq!(calculate_max_exposure(60.0), 15999.0);
    }

    #[test]
    fn test_adjust_exposure() {
        assert_eq!(adjust_exposure(10000.0, 30.0), 10000.0);
        assert_eq!(adjust_exposure(40000.0, 30.0), 32999.0);
    }

    #[test]
    fn test_calculate_resolution_offset() {
        assert_eq!(calculate_resolution_offset((2048, 2048), 1024, 1024), (512, 512));
        assert_eq!(calculate_resolution_offset((2048, 2048), 2048, 2048), (0, 0));
    }
}