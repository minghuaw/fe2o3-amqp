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

pub(crate) use mux::{SessionLocalOption, SessionMux};

/// Default incoming_window and outgoing_window
pub const DEFAULT_WINDOW: UInt = 100;

pub struct SessionFrame {
    channel: u16, // outgoing/local channel number
    body: SessionFrameBody,
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

    MappedEndSent,

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
