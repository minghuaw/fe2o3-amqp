use bytes::BytesMut;
use fe2o3_amqp_types::performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer};


/// A subset of AMQP frames that should be handled or intercepted by
/// a Session endpoint.
pub struct SessionFrame {
    pub channel: u16, // outgoing/local channel number
    pub body: SessionFrameBody,
}

impl SessionFrame {
    pub fn new(channel: impl Into<u16>, body: impl Into<SessionFrameBody>) -> Self {
        Self {
            channel: channel.into(),
            body: body.into()
        }
    }
}

pub enum SessionFrameBody {
    Begin(Begin),
    Attach(Attach),
    Flow(Flow),
    Transfer {
        performative: Transfer,
        payload: Option<BytesMut>,
    },
    Disposition(Disposition),
    Detach(Detach),
    End(End),
}