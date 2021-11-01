//! Controls for Connection, Session, and Link

use fe2o3_amqp_types::definitions::Error;
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{connection::engine::SessionId, session::SessionIncomingItem};

pub enum ConnectionControl {
    Open,
    Close(Option<Error>),
    CreateSession {
        tx: Sender<SessionIncomingItem>,
        responder: oneshot::Sender<(u16, SessionId)>,
    },
    DropSession(SessionId),
}

pub enum SessionControl {
    Begin,
    End(Option<Error>),
    CreateLink,
    DropLink,
}

pub enum LinkControl {}
