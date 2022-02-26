//! Controls for Connection, Session, and Link

use fe2o3_amqp_types::{
    definitions::{self, Handle},
    performatives::Disposition,
};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    connection::{engine::SessionId, AllocSessionError},
    endpoint::LinkFlow,
    link::LinkHandle,
    session::{AllocLinkError, SessionIncomingItem},
};

#[derive(Debug)]
pub enum ConnectionControl {
    Open,
    Close(Option<definitions::Error>),
    AllocateSession {
        tx: Sender<SessionIncomingItem>,
        responder: oneshot::Sender<Result<(u16, SessionId), AllocSessionError>>,
    },
    DeallocateSession(SessionId),
}

impl std::fmt::Display for ConnectionControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "Open"),
            Self::Close(err) => write!(f, "Close({:?})", err),
            Self::AllocateSession {
                tx: _,
                responder: _,
            } => write!(f, "AllocateSession"),
            Self::DeallocateSession(id) => write!(f, "DeallocateSession({})", id),
        }
    }
}

pub enum SessionControl {
    Begin,
    End(Option<definitions::Error>),
    AllocateLink {
        link_name: String,
        link_handle: LinkHandle,
        responder: oneshot::Sender<Result<Handle, AllocLinkError>>,
    },
    DeallocateLink(String),
    LinkFlow(LinkFlow),
    Disposition(Disposition),
}

pub enum LinkControl {}
