use super::structs::{KalmanEstimateRow, MessageType};
use crossbeam::channel;

pub fn connect_to_socket(port: &str, socket_type: zmq::SocketType) -> zmq::Socket {
    let context = zmq::Context::new();
    let socket = context.socket(socket_type).unwrap();
    socket
        .connect(format!("tcp://127.0.0.1:{}", port).as_str())
        .unwrap();
    if socket_type == zmq::SUB {
        socket.set_subscribe(b"trigger").unwrap();
    };
    socket
}

pub fn parse_message(message: &str) -> MessageType {
    if message.trim().is_empty() {
        return MessageType::Empty;
    }

    match serde_json::from_str::<KalmanEstimateRow>(message) {
        Ok(data) => MessageType::JsonData(data),
        Err(e) => {
            if e.is_data() {
                // If the error is due to data format issues, return InvalidJson
                MessageType::InvalidJson(message.to_string(), e)
            } else {
                // For other types of errors, treat it as a plain text message
                MessageType::Text(message.to_string())
            }
        }
    }
}

pub fn subscribe_to_messages(subscriber: zmq::Socket, msg_sender: channel::Sender<String>) {
    loop {
        let msg = match subscriber.recv_string(zmq::DONTWAIT) {
            Ok(result) => match result {
                Ok(full_message) => {
                    let parts: Vec<&str> = full_message.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let topic = parts[0];
                        let message = parts[1];
                        log::info!("Received message: {:?} {:?}", topic, message);
                        Some(message.to_string())
                    } else {
                        log::warn!("Received message with no topic: {:?}", full_message);
                        Some(full_message)
                    }
                }
                Err(e) => {
                    log::trace!("Failed to parse message: {:?}", e);
                    None
                }
            },
            Err(e) => {
                log::trace!("Failed to receive message: {:?}", e);
                None
            }
        };

        if let Some(message) = msg {
            if let Err(e) = msg_sender.send(message.clone()) {
                log::error!("Failed to send message to main thread: {:?}", e);
                break;
            }

            if message == "kill" {
                log::info!("Kill message received, stopping subscriber thread.");
                break;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(1)); // Adjusted to check every 1ms
    }
}
