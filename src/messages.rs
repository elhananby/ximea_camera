use crate::structs::{KalmanEstimateRow, MessageType};
use anyhow::{Context, Result};
use crossbeam::channel;
use std::time::Duration;
use zmq::{Context as ZmqContext, Socket, SocketType};

pub fn connect_to_socket(port: &str, socket_type: SocketType) -> Result<Socket> {
    let context = ZmqContext::new();
    let socket = context.socket(socket_type)
        .context("Failed to create ZMQ socket")?;
    
    socket.connect(&format!("tcp://127.0.0.1:{}", port))
        .with_context(|| format!("Failed to connect to ZMQ socket on port {}", port))?;
    
    if socket_type == SocketType::SUB {
        socket.set_subscribe(b"trigger")
            .context("Failed to set ZMQ subscription")?;
    }
    
    Ok(socket)
}

pub fn parse_message(message: &str) -> MessageType {
    if message.trim().is_empty() {
        return MessageType::Empty;
    }

    match serde_json::from_str::<KalmanEstimateRow>(message) {
        Ok(data) => MessageType::JsonData(data),
        Err(e) if e.is_data() => MessageType::InvalidJson(message.to_string(), e),
        Err(_) => MessageType::Text(message.to_string()),
    }
}

pub fn subscribe_to_messages(subscriber: Socket, msg_sender: channel::Sender<String>) -> Result<()> {
    let mut message_buffer = Vec::new();

    loop {
        match subscriber.recv_multipart(zmq::DONTWAIT) {
            Ok(multipart_msg) => {
                message_buffer.extend(multipart_msg);
                while let Some(index) = message_buffer.iter().position(|part| part.is_empty()) {
                    let full_message = message_buffer.drain(..=index)
                        .filter(|part| !part.is_empty())
                        .map(|part| String::from_utf8_lossy(&part).into_owned())
                        .collect::<Vec<String>>()
                        .join(" ");
                    
                    if !full_message.is_empty() {
                        log::info!("Received message: {}", full_message);
                        msg_sender.send(full_message.clone())
                            .context("Failed to send message to main thread")?;
                        
                        if full_message == "kill" {
                            log::info!("Kill message received, stopping subscriber thread.");
                            return Ok(());
                        }
                    }
                }
            }
            Err(zmq::Error::EAGAIN) => {
                // No message available, sleep for a short duration
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) => {
                log::error!("Failed to receive message: {}", e);
                return Err(e).context("Error in ZMQ receive");
            }
        }
    }
}