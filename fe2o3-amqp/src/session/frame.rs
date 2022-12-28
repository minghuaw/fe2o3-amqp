use fe2o3_amqp_types::performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer};

use crate::Payload;

pub(crate) type SessionIncomingItem = SessionFrame;

pub(crate) enum SessionOutgoingItem {
    SingleFrame(SessionFrame),
    MultipleFrames(Vec<SessionFrame>),
}

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

// #[derive(Debug)]
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

impl std::fmt::Debug for SessionFrameBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attach(arg0) => f.debug_tuple("Attach").field(arg0).finish(),
            Self::Flow(arg0) => f.debug_tuple("Flow").field(arg0).finish(),
            Self::Transfer {
                performative,
                payload,
            } => f
                .debug_struct("Transfer")
                .field("performative", performative)
                .field("payload.len", &payload.len())
                .finish(),
            Self::Disposition(arg0) => f.debug_tuple("Disposition").field(arg0).finish(),
            Self::Detach(arg0) => f.debug_tuple("Detach").field(arg0).finish(),
            Self::Begin(arg0) => f.debug_tuple("Begin").field(arg0).finish(),
            Self::End(arg0) => f.debug_tuple("End").field(arg0).finish(),
        }
    }
}
