//! Error types for session operations

use fe2o3_amqp_types::definitions::{self};

use crate::link::LinkRelayError;

/// Error with ending a session
#[derive(Debug, thiserror::Error)]
pub(crate) enum SessionStateError {
    /// Illegal session state
    #[error("Illegal session state")]
    IllegalState,

    /// The associated connection must have been closed
    #[error("Connection must have been closed")]
    IllegalConnectionState,

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

    /// The associated connection must have been closed
    #[error("Connection must have been closed")]
    IllegalConnectionState,

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

impl From<SessionStateError> for BeginError {
    fn from(error: SessionStateError) -> Self {
        match error {
            SessionStateError::IllegalState => Self::IllegalState,
            SessionStateError::IllegalConnectionState => Self::IllegalConnectionState,
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

    /// The associated connection must have been closed
    #[error("Connection must have been closed")]
    IllegalConnectionState,

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
            SessionStateError::IllegalConnectionState => Self::IllegalConnectionState,
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

    /// The associated connection must have been closed
    #[error("Connection must have been closed")]
    IllegalConnectionState,

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
            SessionInnerError::IllegalConnectionState => Self::IllegalConnectionState,
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
            SessionStateError::IllegalConnectionState => Self::IllegalConnectionState,
            SessionStateError::RemoteEnded => Self::RemoteEnded,
            SessionStateError::RemoteEndedWithError(err) => Self::RemoteEndedWithError(err),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AllocLinkError {
    #[error("Illegal session state")]
    IllegalSessionState,

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
