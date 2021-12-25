use bytes::BytesMut;
use fe2o3_amqp_types::{
    definitions::SequenceNo,
    messaging::Message,
    performatives::{Attach, Detach, Disposition, Flow, Transfer},
    primitives::{Boolean, UInt},
};

pub type LinkIncomingItem = LinkFrame;

#[derive(Debug)]
pub enum LinkFrame {
    Attach(Attach),
    Flow(Flow),
    Transfer {
        performative: Transfer,
        message: Message,
    },
    Disposition(Disposition),
    Detach(Detach),
}
