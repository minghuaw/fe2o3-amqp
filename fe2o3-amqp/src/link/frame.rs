use bytes::{Bytes, BytesMut};
use fe2o3_amqp_types::performatives::{Attach, Detach, Disposition, Flow, Transfer};

use crate::endpoint::LinkFlow;

pub type LinkIncomingItem = LinkFrame;

#[derive(Debug)]
pub enum LinkFrame {
    Attach(Attach),
    Flow(LinkFlow),
    Transfer {
        performative: Transfer,
        payload: BytesMut,
    },
    Disposition(Disposition),
    Detach(Detach),
}
