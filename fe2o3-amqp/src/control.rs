//! Controls for Connection, Session, and Link

use fe2o3_amqp_types::definitions::{Error, Handle};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{connection::engine::SessionId, error::EngineError, link::LinkIncomingItem, session::SessionIncomingItem};

pub enum ConnectionControl {
    Open,
    Close(Option<Error>),
    CreateSession {
        tx: Sender<SessionIncomingItem>,
        responder: oneshot::Sender<Result<(u16, SessionId), EngineError>>,
    },
    DropSession(SessionId),
}

pub enum SessionControl {
    Begin,
    End(Option<Error>),
    CreateLink{
        tx: Sender<LinkIncomingItem>,
        responder: oneshot::Sender<Result<Handle, EngineError>>,
    },
    DropLink(Handle),
}

pub enum LinkControl {}
