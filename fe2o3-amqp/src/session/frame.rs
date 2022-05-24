use fe2o3_amqp_types::performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer};

use crate::Payload;

pub(crate) type SessionIncomingItem = SessionFrame;

/// A subset of AMQP frames that should be handled or intercepted by
/// a Session endpoint.
#[derive(Debug)]
pub(crate) struct SessionFrame {
    pub channel: u16, // outgoing/local channel number
    pub body: SessionFrameBody,
}

impl SessionFrame {
    pub fn new(channel: impl Into<u16>, body: SessionFrameBody) -> Self {
        Self {
            channel: channel.into(),
            body,
        }
    }
}

#[derive(Debug)]
pub(crate) enum SessionFrameBody {
    // Frames handled by Link
    Attach(Attach),
    Flow(Flow),
    Transfer {
        performative: Transfer,
        payload: Payload,
    },
    Disposition(Disposition),
    Detach(Detach),

    // Frames handled by Session
    Begin(Begin),
    End(End),
}
