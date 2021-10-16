use std::convert::TryFrom;

use bytes::BytesMut;
use serde_amqp::primitives::UInt;
use fe2o3_amqp_types::performatives::{Attach, Begin, Close, Detach, Disposition, End, Flow, Open, Transfer};
use tokio::{sync::mpsc::Sender, task::JoinHandle};

use crate::error::EngineError;

use self::{builder::Builder, mux::SessionMuxControl};

use super::{amqp::{Frame, FrameBody}, connection::{Connection, OutgoingChannelId}};

mod builder;
mod mux;

pub(crate) use mux::SessionMux;

/// Default incoming_window and outgoing_window
pub const DEFAULT_WINDOW: UInt = 2048;

pub(crate) struct SessionFrame {
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

pub(crate) enum SessionFrameBody {
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

pub(crate) struct NonSessionFrame {
    pub channel: u16,
    pub body: NonSessionFrameBody,
}

pub(crate) enum NonSessionFrameBody {
    Open(Open),
    Close(Close),
    // An empty frame used only for heartbeat
    Empty,
}

impl NonSessionFrameBody {
    pub fn open(performative: Open) -> Self {
        Self::Open (performative)
    }

    pub fn close(performative: Close) -> Self {
        Self::Close (performative)
    }

    pub fn empty() -> Self {
        Self::Empty
    }
}

impl From<SessionFrame> for Frame {
    fn from(value: SessionFrame) -> Self {
        let channel = value.channel;
        let body = match value.body {
            SessionFrameBody::Begin(performative) => FrameBody::begin(performative),
            SessionFrameBody::Attach(performative) => FrameBody::attach(performative),
            SessionFrameBody::Flow(performative) => FrameBody::flow(performative),
            SessionFrameBody::Transfer{performative, payload} => FrameBody::transfer(performative, payload),
            SessionFrameBody::Disposition(performative) => FrameBody::disposition(performative),
            SessionFrameBody::Detach(performative) => FrameBody::detach(performative),
            SessionFrameBody::End(performative) => FrameBody::end(performative)
        };
        Self::new(channel, body)
    }
}

impl TryFrom<Frame> for SessionFrame {
    type Error = NonSessionFrame;

    fn try_from(value: Frame) -> Result<Self, Self::Error> {
        let channel = value.channel;

        let body = match value.body {
            FrameBody::Begin(performative) => SessionFrameBody::begin(performative),
            FrameBody::Attach(performative) => SessionFrameBody::attach(performative),
            FrameBody::Flow(performative) => SessionFrameBody::flow(performative),
            FrameBody::Transfer{performative, payload} => SessionFrameBody::transfer(performative, payload),
            FrameBody::Disposition(performative) => SessionFrameBody::disposition(performative),
            FrameBody::Detach(performative) => SessionFrameBody::detach(performative),
            FrameBody::End(performative) => SessionFrameBody::end(performative),
            FrameBody::Open(performative) => return Err(NonSessionFrame{
                channel, 
                body: NonSessionFrameBody::open(performative)
            }),
            FrameBody::Close(performative) => return Err(NonSessionFrame{
                channel,
                body: NonSessionFrameBody::close(performative)
            }),
            FrameBody::Empty => return Err(NonSessionFrame{
                channel,
                body: NonSessionFrameBody::empty()
            })
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

    pub fn sender(&self) -> &Sender<Result<SessionFrame, EngineError>> {
        &self.sender
    }

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

    pub async fn begin(connection: &mut Connection) -> Result<Self, EngineError> {
        Self::builder()
            .begin(connection).await
    }

    pub async fn end(&mut self) -> Result<(), EngineError> {
        self.mux.send(mux::SessionMuxControl::End).await?;
        match (&mut self.handle).await {
            Ok(res) => res,
            Err(_) => Err(EngineError::Message("JoinError")),
        }
    }
}
