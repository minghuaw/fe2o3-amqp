use bytes::BytesMut;
use fe2o3_types::performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer};
use tokio::sync::mpsc::Sender;

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
    tx: Sender<SessionFrame>,
}

pub enum SessionState {

}

pub struct Session {
    id: OutChanId,
}