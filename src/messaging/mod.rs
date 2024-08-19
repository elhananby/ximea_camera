mod zmq_handler;
mod message_types;

pub use zmq_handler::ZmqHandler;
pub use message_types::{Message, MessageType, KalmanEstimateRow};

pub trait MessageHandler {
    fn connect(&mut self) -> Result<(), crate::error::Error>;
    fn receive_message(&mut self) -> Result<Option<Message>, crate::error::Error>;
    fn send_message(&mut self, message: &Message) -> Result<(), crate::error::Error>;
}