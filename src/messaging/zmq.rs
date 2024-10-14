use super::types::{KalmanEstimateRow, MessageType};
use anyhow::{Context, Result};
use log::{debug, error, info, trace};
use serde_json;
use std::time::Duration;
use zmq;

pub struct ZmqSubscriber {
    subscriber: zmq::Socket,
}

impl ZmqSubscriber {
    pub fn new(address: &str, port: &str) -> Result<Self> {
        let context = zmq::Context::new();
        let subscriber = context.socket(zmq::SUB)?;
        subscriber.connect(&format!("tcp://{}:{}", address, port))?;
        subscriber.set_subscribe(b"trigger")?;
        Ok(Self { subscriber })
    }

    pub fn receive_message(&self, timeout: Duration) -> Result<MessageType> {
        self.subscriber.set_rcvtimeo(timeout.as_millis() as i32)?;

        match self.subscriber.recv_string(zmq::DONTWAIT) {
            Ok(Ok(message)) => {
                let parts: Vec<&str> = message.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let topic = parts[0];
                    let payload = parts[1];
                    debug!("Received message: Topic={}, Payload={}", topic, payload);
                    self.parse_message(payload)
                } else {
                    error!("Received message with unexpected format: {}", message);
                    Ok(MessageType::Text(message))
                }
            }
            Ok(Err(e)) => {
                error!("Failed to parse message: {:?}", e);
                Ok(MessageType::Empty)
            }
            Err(zmq::Error::EAGAIN) => {
                trace!("No message received within timeout");
                Ok(MessageType::Empty)
            }
            Err(e) => {
                error!("ZMQ error: {:?}", e);
                Err(e).context("Failed to receive ZMQ message")
            }
        }
    }

    fn parse_message(&self, message: &str) -> Result<MessageType> {
        if message.is_empty() {
            return Ok(MessageType::Empty);
        }

        match serde_json::from_str::<KalmanEstimateRow>(message) {
            Ok(data) => Ok(MessageType::JsonData(data)),
            Err(e) => {
                if e.is_data() {
                    Ok(MessageType::InvalidJson(
                        message.to_string(),
                        e.to_string(),
                    ))
                } else {
                    Ok(MessageType::Text(message.to_string()))
                }
            }
        }
    }
}