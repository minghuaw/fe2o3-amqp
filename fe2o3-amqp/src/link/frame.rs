use bytes::BytesMut;
use fe2o3_amqp_types::{definitions::SequenceNo, performatives::{Attach, Detach, Disposition, Flow, Transfer}, primitives::{Boolean, UInt}};

pub type LinkIncomingItem = LinkFrame;

pub enum LinkFrame {
    Attach(Attach),
    Flow(Flow),
    Transfer {
        performative: Transfer,
        payload: Option<BytesMut>,
    },
    Disposition(Disposition),
    Detach(Detach),
}

