use bytes::BytesMut;
use fe2o3_amqp_types::performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer};

// pub type SessionIncomingItem = Result<SessionFrame, connection::Error>;
pub type SessionIncomingItem = SessionFrame;

/// A subset of AMQP frames that should be handled or intercepted by
/// a Session endpoint.
#[derive(Debug)]
pub struct SessionFrame {
    pub channel: u16, // outgoing/local channel number
    pub body: SessionFrameBody,
}

impl SessionFrame {
    pub fn new(channel: impl Into<u16>, body: impl Into<SessionFrameBody>) -> Self {
        Self {
            channel: channel.into(),
            body: body.into(),
        }
    }
}

#[derive(Debug)]
pub enum SessionFrameBody {
    // Frames handled by Link
    Attach(Attach),
    Flow(Flow),
    Transfer {
        performative: Transfer,
        payload: BytesMut,
    },
    Disposition(Disposition),
    Detach(Detach),

    // Frames handled by Session
    Begin(Begin),
    End(End),
}
