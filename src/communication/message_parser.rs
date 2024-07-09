use anyhow::{Result, Context, bail};
use serde_json;

use crate::types::TriggerMessage;

pub fn parse_message(message: &str) -> Result<TriggerMessage> {
    let parts: Vec<&str> = message.splitn(2, ' ').collect();
    
    if parts.len() != 2 || parts[0] != "trigger" {
        bail!("Invalid message format");
    }

    let payload = parts[1];
    let trigger: TriggerMessage = serde_json::from_str(payload)
        .context("Failed to parse JSON payload")?;

    Ok(trigger)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_message() {
        let message = r#"trigger {"obj_id": 1, "frame": 100, "timestamp": 1234567890.123}"#;
        let result = parse_message(message);
        assert!(result.is_ok());
        let trigger = result.unwrap();
        assert_eq!(trigger.obj_id, 1);
        assert_eq!(trigger.frame, 100);
        assert_eq!(trigger.timestamp, 1234567890.123);
    }

    #[test]
    fn test_invalid_format() {
        let message = r#"invalid {"obj_id": 1, "frame": 100, "timestamp": 1234567890.123}"#;
        let result = parse_message(message);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_json() {
        let message = r#"trigger {"obj_id": 1, "frame": 100, "timestamp": 1234567890.123"#;
        let result = parse_message(message);
        assert!(result.is_err());
    }
}