//! Controls for Connection, Session, and Link

use fe2o3_amqp_types::{
    definitions::{self, Handle},
    performatives::Disposition,
};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    connection::{AllocSessionError},
    endpoint::{LinkFlow, OutgoingChannel},
    link::LinkHandle,
    session::{frame::SessionIncomingItem, AllocLinkError},
};

#[derive(Debug)]
pub(crate) enum ConnectionControl {
    // Open,
    Close(Option<definitions::Error>),
    AllocateSession {
        tx: Sender<SessionIncomingItem>,
        responder: oneshot::Sender<Result<OutgoingChannel, AllocSessionError>>,
    },
    DeallocateSession(OutgoingChannel),
}

impl std::fmt::Display for ConnectionControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Self::Open => write!(f, "Open"),
            Self::Close(err) => write!(f, "Close({:?})", err),
            Self::AllocateSession {
                tx: _,
                responder: _,
            } => write!(f, "AllocateSession"),
            Self::DeallocateSession(id) => write!(f, "DeallocateSession({})", id.0),
        }
    }
}

#[allow(dead_code)]
pub(crate) enum SessionControl {
    End(Option<definitions::Error>),
    AllocateLink {
        link_name: String,
        link_handle: LinkHandle,
        responder: oneshot::Sender<Result<Handle, AllocLinkError>>,
    },
    AllocateIncomingLink {
        link_name: String,
        link_handle: LinkHandle,
        input_handle: Handle,
        responder: oneshot::Sender<Result<Handle, AllocLinkError>>,
    },
    DeallocateLink(String),
    LinkFlow(LinkFlow),
    Disposition(Disposition),
}

impl std::fmt::Display for SessionControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionControl::End(_) => write!(f, "End"),
            SessionControl::AllocateLink {
                link_name: _,
                link_handle: _,
                responder: _,
            } => write!(f, "AllocateLink"),
            SessionControl::AllocateIncomingLink {
                link_name: _,
                link_handle: _,
                input_handle: _,
                responder: _,
            } => write!(f, "AllocateIncomingLink"),
            SessionControl::DeallocateLink(name) => write!(f, "DeallocateLink({})", name),
            SessionControl::LinkFlow(_) => write!(f, "LinkFlow"),
            SessionControl::Disposition(_) => write!(f, "Disposition"),
        }
    }
}
