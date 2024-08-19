use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_type: MessageType,
    pub content: String,
}

impl Message {
    pub fn new(content: String) -> Self {
        // For now, we'll assume all messages are Text type
        // In a real application, you'd want to parse the content and determine the correct type
        Self {
            message_type: MessageType::Text,
            content,
        }
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text,
    JsonData,
    Kill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KalmanEstimateRow {
    pub obj_id: u32,
    pub frame: u64,
    pub timestamp: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub xvel: f64,
    pub yvel: f64,
    pub zvel: f64,
    // Add other fields as necessary
}