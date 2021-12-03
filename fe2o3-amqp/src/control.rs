//! Controls for Connection, Session, and Link

use fe2o3_amqp_types::definitions::{self, Handle};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    connection::{engine::SessionId, self}, endpoint::LinkFlow, error::EngineError, link::LinkHandle,
    session::SessionIncomingItem,
};

#[derive(Debug)]
pub enum ConnectionControl {
    Open,
    Close(Option<definitions::Error>),
    CreateSession {
        tx: Sender<SessionIncomingItem>,
        responder: oneshot::Sender<Result<(u16, SessionId), connection::AllocSessionError>>,
    },
    DropSession(SessionId),
}

pub enum SessionControl {
    Begin,
    End(Option<definitions::Error>),
    CreateLink {
        link_handle: LinkHandle,
        responder: oneshot::Sender<Result<Handle, EngineError>>,
    },
    DropLink(Handle),
    LinkFlow(LinkFlow),
}

pub enum LinkControl {}
