use fe2o3_amqp::{Receiver, Sender};

use crate::projected::TryIntoProjected;

#[derive(Debug)]
pub struct Client {
    sender: Sender,
    receiver: Receiver,
}
