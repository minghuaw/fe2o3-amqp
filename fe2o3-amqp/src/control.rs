//! Controls for Connection, Session, and Link

use fe2o3_amqp_types::{
    definitions::{self, ConnectionError},
    performatives::Disposition,
};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    connection::AllocSessionError,
    endpoint::{InputHandle, OutgoingChannel, OutputHandle},
    link::LinkRelay,
    session::{frame::SessionIncomingItem, AllocLinkError},
};

#[cfg(feature = "transaction")]
use fe2o3_amqp_types::{
    messaging::Accepted, transaction::TransactionError, transaction::TransactionId,
};

#[cfg(feature = "transaction")]
use crate::transaction::AllocTxnIdError;

#[derive(Debug)]
pub(crate) enum ConnectionControl {
    // Open,
    Close(Option<definitions::Error>),
    AllocateSession {
        tx: Sender<SessionIncomingItem>,
        responder: oneshot::Sender<Result<OutgoingChannel, AllocSessionError>>,
    },
    DeallocateSession(OutgoingChannel),
    GetMaxFrameSize(oneshot::Sender<usize>),
}

impl std::fmt::Display for ConnectionControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Close(err) => write!(f, "Close({:?})", err),
            Self::AllocateSession {
                tx: _,
                responder: _,
            } => write!(f, "AllocateSession"),
            Self::DeallocateSession(id) => write!(f, "DeallocateSession({})", id.0),
            Self::GetMaxFrameSize(_) => write!(f, "GetMaxFrameSize"),
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
    Disposition(Disposition),
    CloseConnectionWithError((ConnectionError, Option<String>)),
    GetMaxFrameSize(oneshot::Sender<usize>),

    // Transaction related controls
    #[cfg(feature = "transaction")]
    AllocateTransactionId {
        resp: oneshot::Sender<Result<TransactionId, AllocTxnIdError>>,
    },
    #[cfg(feature = "transaction")]
    CommitTransaction {
        txn_id: TransactionId,
        resp: oneshot::Sender<Result<Accepted, TransactionError>>,
    },
    #[cfg(feature = "transaction")]
    RollbackTransaction {
        txn_id: TransactionId,
        resp: oneshot::Sender<Result<Accepted, TransactionError>>,
    },
    #[cfg(feature = "transaction")]
    AbortTransaction(TransactionId),
}

impl std::fmt::Display for SessionControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionControl::End(err) => write!(f, "End({:?})", err),
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
            SessionControl::Disposition(_) => write!(f, "Disposition"),
            SessionControl::CloseConnectionWithError(_) => write!(f, "CloseConnectionWithError"),
            SessionControl::GetMaxFrameSize(_) => write!(f, "GetMaxFrameSize"),

            #[cfg(feature = "transaction")]
            SessionControl::AllocateTransactionId { .. } => write!(f, "AllocateTransactionId"),
            #[cfg(feature = "transaction")]
            SessionControl::CommitTransaction { .. } => write!(f, "CommitTransaction"),
            #[cfg(feature = "transaction")]
            SessionControl::RollbackTransaction { .. } => write!(f, "RollbackTransaction"),
            #[cfg(feature = "transaction")]
            SessionControl::AbortTransaction(_) => write!(f, "AbortTransaction"),
        }
    }
}
