use fe2o3_amqp::{Receiver, Sender};

#[derive(Debug)]
pub struct Client {
    sender: Sender,
    receiver: Receiver,
}