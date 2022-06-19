//! Controls for Connection, Session, and Link

use fe2o3_amqp_types::{
    definitions::{self, ConnectionError},
    performatives::Disposition,
};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    connection::AllocSessionError,
    endpoint::{InputHandle, LinkFlow, OutgoingChannel, OutputHandle},
    link::LinkRelay,
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
        link_relay: LinkRelay<()>,
        responder: oneshot::Sender<Result<OutputHandle, AllocLinkError>>,
    },
    AllocateIncomingLink {
        link_name: String,
        link_relay: LinkRelay<()>,
        input_handle: InputHandle,
        responder: oneshot::Sender<Result<OutputHandle, AllocLinkError>>,
    },
    DeallocateLink(OutputHandle),
    LinkFlow(LinkFlow),
    Disposition(Disposition),
    CloseConnectionWithError((ConnectionError, Option<String>)),
}

impl std::fmt::Display for SessionControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionControl::End(_) => write!(f, "End"),
            SessionControl::AllocateLink {
                link_name: _,
                link_relay: _,
                responder: _,
            } => write!(f, "AllocateLink"),
            SessionControl::AllocateIncomingLink {
                link_name: _,
                link_relay: _,
                input_handle: _,
                responder: _,
            } => write!(f, "AllocateIncomingLink"),
            SessionControl::DeallocateLink(name) => write!(f, "DeallocateLink({:?})", name),
            SessionControl::LinkFlow(_) => write!(f, "LinkFlow"),
            SessionControl::Disposition(_) => write!(f, "Disposition"),
            SessionControl::CloseConnectionWithError(_) => write!(f, "CloseConnectionWithError"),
        }
    }
}
