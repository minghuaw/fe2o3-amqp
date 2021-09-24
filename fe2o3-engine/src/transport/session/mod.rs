use std::convert::TryFrom;

use bytes::BytesMut;
use fe2o3_amqp::primitives::UInt;
use fe2o3_types::performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer};
use tokio::{sync::mpsc::Sender, task::JoinHandle};

use crate::error::EngineError;

use self::{builder::Builder, mux::SessionMuxControl};

use super::{
    amqp::{Frame, FrameBody},
    connection::OutgoingChannelId,
};

mod builder;
mod mux;

pub(crate) use mux::SessionMux;

/// Default incoming_window and outgoing_window
pub const DEFAULT_WINDOW: UInt = 100;

pub struct SessionFrame {
    channel: u16, // outgoing/local channel number
    body: SessionFrameBody,
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
    Begin {
        performative: Begin,
    },
    Attach {
        performative: Attach,
    },
    Flow {
        performative: Flow,
    },
    Transfer {
        performative: Transfer,
        payload: Option<BytesMut>,
    },
    Disposition {
        performative: Disposition,
    },
    Detach {
        performative: Detach,
    },
    End {
        performative: End,
    },
}

impl SessionFrameBody {
    pub fn begin(performative: Begin) -> Self {
        Self::Begin { performative }
    }

    pub fn attach(performative: Attach) -> Self {
        Self::Attach { performative }
    }

    pub fn flow(performative: Flow) -> Self {
        Self::Flow { performative }
    }

    pub fn transfer(performative: Transfer, payload: Option<BytesMut>) -> Self {
        Self::Transfer {
            performative,
            payload,
        }
    }

    pub fn disposition(performative: Disposition) -> Self {
        Self::Disposition { performative }
    }

    pub fn detach(performative: Detach) -> Self {
        Self::Detach { performative }
    }

    pub fn end(performative: End) -> Self {
        Self::End { performative }
    }
}

impl From<SessionFrame> for Frame {
    fn from(value: SessionFrame) -> Self {
        let channel = value.channel;
        let body = match value.body {
            SessionFrameBody::Begin{performative} => FrameBody::begin(performative),
            SessionFrameBody::Attach{performative} => FrameBody::attach(performative),
            SessionFrameBody::Flow{performative} => FrameBody::flow(performative),
            SessionFrameBody::Transfer{performative, payload} => FrameBody::transfer(performative, payload),
            SessionFrameBody::Disposition{performative} => FrameBody::disposition(performative),
            SessionFrameBody::Detach{performative} => FrameBody::detach(performative),
            SessionFrameBody::End{performative} => FrameBody::end(performative)
        };
        Self::new(channel, body)
    }
}

impl TryFrom<Frame> for SessionFrame {
    type Error = Frame;

    fn try_from(value: Frame) -> Result<Self, Self::Error> {
        let channel = value.channel;

        let body = match value.body {
            FrameBody::Begin{performative} => SessionFrameBody::begin(performative),
            FrameBody::Attach{performative} => SessionFrameBody::attach(performative),
            FrameBody::Flow{performative} => SessionFrameBody::flow(performative),
            FrameBody::Transfer{performative, payload} => SessionFrameBody::transfer(performative, payload),
            FrameBody::Disposition{performative} => SessionFrameBody::disposition(performative),
            FrameBody::Detach{performative} => SessionFrameBody::detach(performative),
            FrameBody::End{performative} => SessionFrameBody::end(performative),
            _ => return Err(value)
        };

        Ok(Self {
            channel, 
            body
        })
    }
}

// 2.5.5 Session States
// UNMAPPED
// BEGIN SENT
// BEGIN RCVD
// MAPPED END SENT
// END RCVD
// DISCARDING
pub enum SessionState {
    Unmapped,

    BeginSent,

    BeginReceived,

    Mapped,
    
    EndSent,

    EndReceived,

    Discarding,
}

/// Session handle held by the connection mux
pub(crate) struct SessionHandle {
    // id: OutgoingChannelId,
    sender: Sender<Result<SessionFrame, EngineError>>,
}

impl SessionHandle {
    // pub fn id(&self) -> &OutgoingChannelId {
    //     &self.id
    // }

    pub fn sender_mut(&mut self) -> &mut Sender<Result<SessionFrame, EngineError>> {
        &mut self.sender
    }
}

/// Session API available for public
/// TODO: Only care about creating new local session for now.
///
/// How should a new session be created?
///
pub struct Session {
    mux: Sender<SessionMuxControl>,
    // TODO: send back using a oneshot channel?
    handle: JoinHandle<Result<(), EngineError>>, // JoinHandle to session mux
}

impl Session {
    pub fn builder() -> Builder {
        Builder::new()
    }
}
