//! Error types for session operations

use std::sync::OnceLock;

use fe2o3_amqp_types::definitions::{self};

use crate::{
    connection::{AllocSessionError, ConnectionStopReason},
    link::{LinkRelayError, SessionStopReason},
};

/// Error with ending a session
#[derive(Debug, thiserror::Error)]
pub(crate) enum SessionStateError {
    /// Illegal session state
    #[error("Illegal session state")]
    IllegalState,

    /// The connection stopped before the operation completed
    #[error("The connection stopped: {:?}", .0)]
    ConnectionStopped(ConnectionStopReason),

    /// Remote session ended
    #[error("Remote session ended")]
    RemoteEnded,

    /// Remote session ended with error
    #[error("Remote ended with error")]
    RemoteEndedWithError(definitions::Error),
}

/// Error with beginning a session
#[derive(Debug, thiserror::Error)]
pub enum BeginError {
    /// Illegal session state
    #[error("Illegal session state")]
    IllegalState,

    /// The connection stopped before the operation completed
    #[error("The connection stopped: {:?}", .0)]
    ConnectionStopped(ConnectionStopReason),

    /// The connection has not been opened yet
    #[error("The connection has not been opened")]
    ConnectionNotOpened,

    /// Remote session ended
    #[error("Remote session ended")]
    RemoteEnded,

    /// Remote session ended with error
    #[error("Remote ended with error")]
    RemoteEndedWithError(definitions::Error),

    /// Channel max reached
    #[error("Local channel-max reached")]
    LocalChannelMaxReached,
}

impl From<AllocSessionError> for BeginError {
    fn from(error: AllocSessionError) -> Self {
        match error {
            AllocSessionError::ConnectionNotOpened => Self::ConnectionNotOpened,
            AllocSessionError::ConnectionStopped(reason) => Self::ConnectionStopped(reason),
            AllocSessionError::ChannelMaxReached => Self::LocalChannelMaxReached,
        }
    }
}

impl From<SessionStateError> for BeginError {
    fn from(error: SessionStateError) -> Self {
        match error {
            SessionStateError::IllegalState => Self::IllegalState,
            SessionStateError::ConnectionStopped(reason) => Self::ConnectionStopped(reason),
            SessionStateError::RemoteEnded => Self::RemoteEnded,
            SessionStateError::RemoteEndedWithError(err) => Self::RemoteEndedWithError(err),
        }
    }
}

/// Error with session operations
#[derive(Debug, thiserror::Error)]
pub(crate) enum SessionInnerError {
    /// A frame (other than attach) was received referencing a handle which is not currently in use of an attached link.
    #[error("A frame (other than attach) was received referencing a handle which is not currently in use of an attached link.")]
    UnattachedHandle,

    #[error("Remote sent an attach with a name that cannot be found locally")]
    RemoteAttachingLinkNameNotFound,

    /// An attach was received using a handle that is already in use for an attached link.
    #[error("An attach was received using a handle that is already in use for an attached link.")]
    HandleInUse,

    /// Illegal sesesion state
    #[error("Illegal session state")]
    IllegalState,

    /// The connection stopped before the operation completed
    #[error("The connection stopped: {:?}", .0)]
    ConnectionStopped(ConnectionStopReason),

    /// Found a Transfer frame sent to a Sender
    #[error("Found Transfer frame being sent to a Sender")]
    TransferFrameToSender,

    /// Remote session ended
    #[error("Remote session ended")]
    RemoteEnded,

    /// Remote session ended with error
    #[error("Remote ended with error")]
    RemoteEndedWithError(definitions::Error),

    /// Unknown transaction ID
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(all(feature = "transaction", feature = "acceptor"))]
    #[error("Unknown transaction ID")]
    UnknownTxnId,
}

impl From<SessionStateError> for SessionInnerError {
    fn from(error: SessionStateError) -> Self {
        match error {
            SessionStateError::IllegalState => Self::IllegalState,
            SessionStateError::ConnectionStopped(reason) => Self::ConnectionStopped(reason),
            SessionStateError::RemoteEnded => Self::RemoteEnded,
            SessionStateError::RemoteEndedWithError(err) => Self::RemoteEndedWithError(err),
        }
    }
}

impl From<LinkRelayError> for SessionInnerError {
    fn from(error: LinkRelayError) -> Self {
        match error {
            LinkRelayError::UnattachedHandle => Self::UnattachedHandle,
            LinkRelayError::TransferFrameToSender => Self::TransferFrameToSender,
        }
    }
}

/// Error with session operations
#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A frame (other than attach) was received referencing a handle which is not currently in use of an attached link.
    #[error("A frame (other than attach) was received referencing a handle which is not currently in use of an attached link.")]
    UnattachedHandle,

    /// A remote attach frame is referring to a link name that is not found locally
    #[error("Remote sent an attach with a name that cannot be found locally")]
    RemoteAttachingLinkNameNotFound,

    /// An attach was received using a handle that is already in use for an attached link.
    #[error("An attach was received using a handle that is already in use for an attached link.")]
    HandleInUse,

    /// Illegal sesesion state
    #[error("Illegal session state")]
    IllegalState,

    /// The connection stopped before the operation completed
    #[error("The connection stopped: {:?}", .0)]
    ConnectionStopped(ConnectionStopReason),

    /// Found a Transfer frame sent to a Sender
    #[error("Found Transfer frame being sent to a Sender")]
    TransferFrameToSender,

    /// Remote session ended
    #[error("Remote session ended")]
    RemoteEnded,

    /// Remote session ended with error
    #[error("Remote ended with error")]
    RemoteEndedWithError(definitions::Error),

    /// Unknown transaction ID
    #[cfg(all(feature = "transaction", feature = "acceptor"))]
    #[error("Unknown transaction ID")]
    UnknownTxnId,
}

impl From<SessionInnerError> for Error {
    fn from(error: SessionInnerError) -> Self {
        match error {
            SessionInnerError::UnattachedHandle => Self::UnattachedHandle,
            SessionInnerError::RemoteAttachingLinkNameNotFound => {
                Self::RemoteAttachingLinkNameNotFound
            }
            SessionInnerError::HandleInUse => Self::HandleInUse,
            SessionInnerError::IllegalState => Self::IllegalState,
            SessionInnerError::ConnectionStopped(reason) => Self::ConnectionStopped(reason),
            SessionInnerError::TransferFrameToSender => Self::TransferFrameToSender,
            SessionInnerError::RemoteEnded => Self::RemoteEnded,
            SessionInnerError::RemoteEndedWithError(err) => Self::RemoteEndedWithError(err),

            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(all(feature = "transaction", feature = "acceptor"))]
            SessionInnerError::UnknownTxnId => Self::UnknownTxnId,
        }
    }
}

impl From<LinkRelayError> for Error {
    fn from(error: LinkRelayError) -> Self {
        match error {
            LinkRelayError::UnattachedHandle => Self::UnattachedHandle,
            LinkRelayError::TransferFrameToSender => {
                unreachable!("A sender should not receive a transfer frame")
            }
        }
    }
}

impl From<SessionStateError> for Error {
    fn from(error: SessionStateError) -> Self {
        match error {
            SessionStateError::IllegalState => Self::IllegalState,
            SessionStateError::ConnectionStopped(reason) => Self::ConnectionStopped(reason),
            SessionStateError::RemoteEnded => Self::RemoteEnded,
            SessionStateError::RemoteEndedWithError(err) => Self::RemoteEndedWithError(err),
        }
    }
}

/// The connection's stop reason, or `Closed` when the cell has not been
/// recorded yet (defensive fallback).
pub(crate) fn connection_stop_reason_or_closed(
    cell: &OnceLock<ConnectionStopReason>,
) -> ConnectionStopReason {
    match cell.get() {
        Some(reason) => reason.clone(),
        None => {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "connection stop reason not recorded; reporting ConnectionStopped(Closed)"
            );
            #[cfg(feature = "log")]
            log::warn!("connection stop reason not recorded; reporting ConnectionStopped(Closed)");
            ConnectionStopReason::Closed
        }
    }
}

/// The session stop reason corresponding to a connection stop
impl From<ConnectionStopReason> for SessionStopReason {
    fn from(reason: ConnectionStopReason) -> Self {
        Self::ConnectionStopped(reason)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AllocLinkError {
    /// The session is not in the `Mapped` state (e.g. not begun, or ending)
    #[error("The session is not in a state that permits link allocation")]
    SessionNotMapped,

    #[error("The session stopped before the link was attached: {:?}", .0)]
    SessionStopped(crate::link::SessionStopReason),

    #[error("Link name must be unique")]
    DuplicatedLinkName,
}

/// Error with attempting to end a session
#[derive(Debug, thiserror::Error)]
pub enum TryEndError {
    /// The session is already ended
    #[error("Session is already ended")]
    AlreadyEnded,

    /// The exchange of end frame is not completed because it has not received a remote end frame
    #[error("The sesssion has not received a remote end frame")]
    RemoteEndNotReceived,
}
