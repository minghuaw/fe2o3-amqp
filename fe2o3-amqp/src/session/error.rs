use fe2o3_amqp_types::definitions::{self};
use tokio::task::JoinError;

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
pub enum SessionBeginError {
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

impl From<SessionStateError> for SessionBeginError {
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
#[derive(Debug, thiserror::Error)]
pub enum SessionErrorKind {
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

    /// Event loop exitted with error
    #[error(transparent)]
    JoinError(#[from] JoinError),
}

impl From<SessionInnerError> for SessionErrorKind {
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
        }
    }
}

impl From<LinkRelayError> for SessionErrorKind {
    fn from(error: LinkRelayError) -> Self {
        match error {
            LinkRelayError::UnattachedHandle => Self::UnattachedHandle,
            LinkRelayError::TransferFrameToSender => todo!(),
        }
    }
}

impl From<SessionStateError> for SessionErrorKind {
    fn from(error: SessionStateError) -> Self {
        match error {
            SessionStateError::IllegalState => Self::IllegalState,
            SessionStateError::IllegalConnectionState => Self::IllegalConnectionState,
            SessionStateError::RemoteEnded => Self::RemoteEnded,
            SessionStateError::RemoteEndedWithError(err) => Self::RemoteEndedWithError(err),
        }
    }
}

// impl<'a> TryFrom<&'a SessionErrorKind> for definitions::Error {
//     type Error = &'a SessionErrorKind;

//     fn try_from(kind: &'a SessionErrorKind) -> Result<Self, Self::Error> {
//         use fe2o3_amqp_types::definitions::{AmqpError, Error, SessionError};

//         match kind {
//             SessionErrorKind::UnattachedHandle => {
//                 Ok(Error::new(SessionError::UnattachedHandle, None, None))
//             }
//             SessionErrorKind::RemoteAttachingLinkNameNotFound => Ok(Error::new(
//                 AmqpError::InternalError,
//                 Some(String::from("Link name is not found")),
//                 None,
//             )),
//             SessionErrorKind::HandleInUse => Ok(Error::new(SessionError::HandleInUse, None, None)),
//             SessionErrorKind::IllegalState => Ok(Error::new(AmqpError::IllegalState, None, None)),
//             SessionErrorKind::IllegalConnectionState => Err(kind),
//             SessionErrorKind::TransferFrameToSender => Ok(Error::new(
//                 AmqpError::NotAllowed,
//                 Some(String::from("Found Transfer frame sent Sender link")),
//                 None,
//             )),
//             SessionErrorKind::RemoteEnded => Err(kind),
//             SessionErrorKind::RemoteEndedWithError(_) => Err(kind),
//             SessionErrorKind::JoinError(_) => Err(kind),
//         }
//     }
// }

#[derive(Debug, thiserror::Error)]
pub(crate) enum AllocLinkError {
    #[error("Illegal session state")]
    IllegalSessionState,

    #[error("Link name must be unique")]
    DuplicatedLinkName,
}
