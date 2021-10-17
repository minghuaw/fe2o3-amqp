use bytes::BytesMut;
use fe2o3_amqp_types::performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer};



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

impl SessionFrameBody {
    pub fn begin(performative: Begin) -> Self {
        Self::Begin (performative)
    }

    pub fn attach(performative: Attach) -> Self {
        Self::Attach (performative)
    }

    pub fn flow(performative: Flow) -> Self {
        Self::Flow (performative)
    }

    pub fn transfer(performative: Transfer, payload: Option<BytesMut>) -> Self {
        Self::Transfer {
            performative,
            payload,
        }
    }

    pub fn disposition(performative: Disposition) -> Self {
        Self::Disposition (performative)
    }

    pub fn detach(performative: Detach) -> Self {
        Self::Detach (performative)
    }

    pub fn end(performative: End) -> Self {
        Self::End (performative)
    }
}
