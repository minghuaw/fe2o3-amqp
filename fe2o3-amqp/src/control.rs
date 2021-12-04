//! Controls for Connection, Session, and Link

use fe2o3_amqp_types::definitions::{self, Handle};
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

pub enum SessionControl {
    Begin,
    End(Option<definitions::Error>),
    AllocateLink {
        link_name: String,
        link_handle: LinkHandle,
        responder: oneshot::Sender<Result<Handle, AllocLinkError>>,
    },
    DeallocateLink(Handle),
    LinkFlow(LinkFlow),
}

pub enum LinkControl {}
