use fe2o3_amqp_types::performatives::{Attach, Detach, Disposition, Transfer};

use crate::{endpoint::LinkFlow, Payload};

pub(crate) type LinkIncomingItem = LinkFrame;

/// Link frames.
/// 
/// This is a subset of the AMPQ frames
#[derive(Debug)]
pub(crate) enum LinkFrame {
    Attach(Attach),
    Flow(LinkFlow),
    Transfer {
        performative: Transfer,
        payload: Payload,
    },
    Disposition(Disposition),
    Detach(Detach),
}
