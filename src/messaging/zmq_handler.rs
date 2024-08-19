use super::{MessageHandler, Message};
use crate::error::Error;
use zmq;

pub struct ZmqHandler {
    context: zmq::Context,
    subscriber: Option<zmq::Socket>,
    publisher: Option<zmq::Socket>,
    sub_port: String,
    pub_port: String,
}

impl ZmqHandler {
    pub fn new(sub_port: String, pub_port: String) -> Self {
        Self {
            context: zmq::Context::new(),
            subscriber: None,
            publisher: None,
            sub_port,
            pub_port,
        }
    }

    fn connect_subscriber(&mut self) -> Result<(), Error> {
        let subscriber = self.context.socket(zmq::SUB)
            .map_err(|e| Error::MessagingError(format!("Failed to create subscriber socket: {}", e)))?;
        
        subscriber.connect(&format!("tcp://127.0.0.1:{}", self.sub_port))
            .map_err(|e| Error::MessagingError(format!("Failed to connect subscriber: {}", e)))?;
        
        subscriber.set_subscribe(b"trigger")
            .map_err(|e| Error::MessagingError(format!("Failed to set subscriber filter: {}", e)))?;

        self.subscriber = Some(subscriber);
        Ok(())
    }

    fn connect_publisher(&mut self) -> Result<(), Error> {
        let publisher = self.context.socket(zmq::PUB)
            .map_err(|e| Error::MessagingError(format!("Failed to create publisher socket: {}", e)))?;
        
        publisher.connect(&format!("tcp://127.0.0.1:{}", self.pub_port))
            .map_err(|e| Error::MessagingError(format!("Failed to connect publisher: {}", e)))?;

        self.publisher = Some(publisher);
        Ok(())
    }
}

impl MessageHandler for ZmqHandler {
    fn connect(&mut self) -> Result<(), Error> {
        self.connect_subscriber()?;
        self.connect_publisher()?;
        Ok(())
    }

    fn receive_message(&mut self) -> Result<Option<Message>, Error> {
        if let Some(ref subscriber) = self.subscriber {
            match subscriber.recv_string(zmq::DONTWAIT) {
                Ok(Ok(message)) => {
                    // Parse the message and return a Message struct
                    // For now, we'll just return the raw string
                    Ok(Some(Message::new(message)))
                },
                Ok(Err(e)) => Err(Error::MessagingError(format!("Failed to parse message: {}", e))),
                Err(zmq::Error::EAGAIN) => Ok(None), // No message available
                Err(e) => Err(Error::MessagingError(format!("Failed to receive message: {}", e))),
            }
        } else {
            Err(Error::MessagingError("Subscriber not connected".to_string()))
        }
    }

    fn send_message(&mut self, message: &Message) -> Result<(), Error> {
        if let Some(ref publisher) = self.publisher {
            publisher.send(message.to_string().as_bytes(), 0)
                .map_err(|e| Error::MessagingError(format!("Failed to send message: {}", e)))
        } else {
            Err(Error::MessagingError("Publisher not connected".to_string()))
        }
    }
}