use anyhow::{Context, Result};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tracing::{error, info};

use super::message_parser::parse_message;
use crate::types::TriggerMessage;
use crate::utils::config::ZmqConfig;

pub struct ZmqClient {
    config: ZmqConfig,
}

impl ZmqClient {
    pub fn new(config: &ZmqConfig) -> Self {
        ZmqClient {
            config: config.clone(),
        }
    }

    pub async fn listen_for_triggers(&self, trigger_sender: Sender<TriggerMessage>) -> Result<()> {
        info!("Starting ZMQ listener");

        let context = zmq::Context::new();
        let subscriber = context
            .socket(zmq::SUB)
            .context("Failed to create ZMQ SUB socket")?;

        subscriber
            .connect(&self.config.sub_address)
            .context("Failed to connect to ZMQ PUB socket")?;
        subscriber
            .set_subscribe(b"trigger")
            .context("Failed to set ZMQ subscription")?;

        info!("Connected to ZMQ server at {}", self.config.sub_address);

        loop {
            if let Ok(msg) = subscriber.recv_string(zmq::DONTWAIT) {
                match msg {
                    Ok(msg_str) => match parse_message(&msg_str) {
                        Ok(trigger) => {
                            if let Err(e) = trigger_sender.send(trigger).await {
                                error!("Failed to send trigger to frame processor: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse trigger message: {}", e);
                        }
                    },
                    Err(e) => {
                        error!("Failed to receive message: {:?}", e);
                    }
                }
            }

            // Sleep for a short duration to avoid busy-waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
