use bytes::BytesMut;
use fe2o3_types::performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer};
use tokio::sync::mpsc::Sender;

use crate::error::EngineError;

use super::{amqp::{Frame, FrameBody}, connection::OutChanId};

pub enum SessionFrame {
    Begin{
        performative: Begin
    },
    Attach{
        performative: Attach
    },
    Flow{
        performative: Flow
    },
    Transfer{
        performative: Transfer,
        payload: Option<BytesMut>,
    },
    Disposition {
        performative: Disposition
    },
    Detach {
        performative: Detach
    },
    End {
        performative: End
    },
}

pub struct SessionHandle {
    id: OutChanId,
    sender: Sender<Result<SessionFrame, EngineError>>,
}

impl SessionHandle {
    pub fn id(&self) -> &OutChanId {
        &self.id
    }

    pub fn sender_mut(&mut self) -> &mut Sender<Result<SessionFrame, EngineError>> {
        &mut self.sender
    }
}

pub enum SessionState {

}

pub struct Session {
    id: OutChanId,
}