use log::{info, warn};
use zmq;
use zmq::{Context, Socket, SocketType};

use super::structs::{MessageType, KalmanEstimateRow};

pub struct Publisher {
    pub_socket: Socket,
    rep_socket: Socket,
}

impl Publisher {
    pub fn new(ctx: &Context, pub_port: u16, handshake_port: u16) -> Self {
        let pub_socket = ctx.socket(SocketType::PUB).unwrap();
        pub_socket.bind(&format!("tcp://*:{pub_port}")).unwrap();

        let rep_socket = ctx.socket(SocketType::REP).unwrap();
        rep_socket
            .bind(&format!("tcp://*:{handshake_port}"))
            .unwrap();

        Publisher {
            pub_socket,
            rep_socket,
        }
    }

    pub fn wait_for_subscriber(&self) {
        loop {
            match self.rep_socket.recv_string(0) {
                Ok(Ok(msg)) if &msg == "Hello" => {
                    self.rep_socket.send("Welcome", 0).unwrap();
                    break;
                }
                Ok(Err(e)) => warn!("Error receiving message: {:?}", e),
                Err(e) => warn!("Communication error: {}", e),
                _ => {}
            }
        }
    }

    pub fn publish(&self, topic: &str, msg: &str) {
        let full_msg = format!("{} {}", topic, msg);
        self.pub_socket.send(full_msg.as_bytes(), 0).unwrap();
        info!("Published message on topic '{}': {}", topic, msg);
    }
}

pub struct Subscriber {
    sub_socket: Socket,
    req_socket: Socket,
}

impl Subscriber {
    pub fn new(ctx: &Context, pub_port: u16, handshake_port: u16, server_ip: &str) -> Self {
        let sub_socket = ctx.socket(SocketType::SUB).unwrap();
        sub_socket
            .connect(&format!("tcp://{server_ip}:{pub_port}"))
            .unwrap();

        let req_socket = ctx.socket(SocketType::REQ).unwrap();
        req_socket
            .connect(&format!("tcp://{server_ip}:{handshake_port}"))
            .unwrap();

        Subscriber {
            sub_socket,
            req_socket,
        }
    }

    pub fn handshake(&self) {
        self.req_socket.send("Hello", 0).unwrap();
        match self.req_socket.recv_string(0) {
            Ok(Ok(reply)) if &reply == "Welcome" => info!("Handshake successful."),
            Ok(Ok(_)) => warn!("Unexpected reply during handshake."),
            Ok(Err(e)) => warn!("Error receiving reply: {:?}", e),
            Err(e) => warn!("Communication error during handshake: {}", e),
        }
    }

    pub fn subscribe(&self, topic: &str) {
        self.sub_socket.set_subscribe(topic.as_bytes()).unwrap();
        info!("Subscribed to topic '{}'.", topic);
    }

    pub fn receive(&self) -> String {
        let msg = self.sub_socket.recv_string(0).unwrap().unwrap();
        info!("Received message: {}", msg);
        msg
    }
}


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
