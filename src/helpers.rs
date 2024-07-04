// Standard library imports
use std::time::{SystemTime, UNIX_EPOCH};

// Current crate and supermodule imports


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
