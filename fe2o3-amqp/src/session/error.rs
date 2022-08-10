use std::io;

use fe2o3_amqp_types::definitions::{self, AmqpError, ErrorCondition, Handle, SessionError};
use tokio::task::JoinError;

use crate::{connection::AllocSessionError, link::LinkRelayError};

#[cfg(feature = "transaction")]
use crate::link::ReceiverAttachError;

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
    RemoteEndedWithError(definitions::Error)
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

    // /// Remote session ended
    // #[error("Remote session ended")]
    // RemoteEnded,

    // /// Remote session ended with error
    // #[error("Remote ended with error")]
    // RemoteEndedWithError(definitions::Error),
}

impl From<LinkRelayError> for SessionInnerError {
    fn from(error: LinkRelayError) -> Self {
        match error {
            LinkRelayError::UnattachedHandle => Self::UnattachedHandle,
            LinkRelayError::TransferFrameToSender => todo!(),
        }
    }
}

// impl From<SessionEndError> for SessionInnerError {
//     fn from(error: SessionEndError) -> Self {
//         match error {
//             SessionEndError::IllegalState => Self::IllegalState,
//             SessionEndError::IllegalConnectionState => Self::IllegalConnectionState,
//             SessionEndError::RemoteEndedWithError(err) => Self::RemoteEndedWithError(err),
//             SessionEndError::RemoteEnded => Self::RemoteEnded,
//         }
//     }
// }

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
            SessionInnerError::RemoteAttachingLinkNameNotFound => Self::RemoteAttachingLinkNameNotFound,
            SessionInnerError::HandleInUse => Self::HandleInUse,
            SessionInnerError::IllegalState => Self::IllegalState,
            SessionInnerError::IllegalConnectionState => Self::IllegalConnectionState,
            SessionInnerError::TransferFrameToSender => Self::TransferFrameToSender,
            // SessionInnerError::RemoteEnded => Self::RemoteEnded,
            // SessionInnerError::RemoteEndedWithError(err) => Self::RemoteEndedWithError(err),
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

// impl From<SessionBeginError> for SessionErrorKind {
//     fn from(error: SessionBeginError) -> Self {
//         match error {
//             SessionBeginError::IllegalState => Self::IllegalState,
//             SessionBeginError::IllegalConnectionState => Self::IllegalConnectionState,
//             SessionBeginError::RemoteEnded => Self::RemoteEnded,
//             SessionBeginError::RemoteEndedWithError(err) => Self::RemoteEndedWithError(err),
//         }
//     }
// }

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

// /// Session errors
// #[derive(Debug, thiserror::Error)]
// pub enum Error {
//     /// Io Error. If includes failing to send to the underlying connection event loop
//     #[error(transparent)]
//     Io(#[from] io::Error),

//     /// An attempt is trying to exceed the maximum number of allowed channel
//     #[error("All channels have been allocated")]
//     ChannelMaxReached,

//     /// Error joining the event loop task. This could occur only when the user attempts
//     /// to end the session
//     #[error(transparent)]
//     JoinError(#[from] JoinError),

//     // /// TODO: Fine grain control over particular errors
//     // ///
//     // /// A local error
//     // #[error("Local error {:?}", .0)]
//     // Local(definitions::Error),
//     /// A peer that receives a handle outside the supported range MUST close the connection with the
//     /// framing-error error-code
//     #[error("A handle outside the supported range is received")]
//     HandleMaxExceeded,

//     /// The remote peer ended the session with the provided error
//     #[error("Remote error {:?}", .0)]
//     Remote(definitions::Error),

//     /// TODO: hide this from public API
//     /// Link handle error should be handled differently. Link handle is only local
//     #[error("Local LinkRelay {:?} error {:?}", .handle, .error)]
//     LinkHandleError {
//         /// Handle of the link
//         handle: Handle,
//         /// Whether the link should close upon having an error
//         closed: bool,
//         /// Error
//         error: definitions::Error,
//     },

//     /// A remotely initiated control link failed to attach
//     ///
//     /// TODO: Hide from public API?
//     #[cfg(feature = "transaction")]
//     #[error("Control link attach error {:?}", .0)]
//     CoordinatorAttachError(ReceiverAttachError),
// }

#[derive(Debug, thiserror::Error)]
pub(crate) enum AllocLinkError {
    #[error("Illegal session state")]
    IllegalSessionState,

    #[error("Link name must be unique")]
    DuplicatedLinkName,
}

// impl From<AllocSessionError> for Error {
//     fn from(err: AllocSessionError) -> Self {
//         match err {
//             AllocSessionError::Io(e) => Self::Io(e),
//             AllocSessionError::ChannelMaxReached => Self::ChannelMaxReached,
//             AllocSessionError::IllegalState => {
//                 Self::Local(definitions::Error::new(AmqpError::IllegalState, None, None))
//             }
//         }
//     }
// }

// impl From<AllocLinkError> for definitions::Error {
//     fn from(err: AllocLinkError) -> Self {
//         match err {
//             AllocLinkError::IllegalSessionState => Self {
//                 condition: AmqpError::IllegalState.into(),
//                 description: Some("Illegal session state".to_string()),
//                 info: None,
//             },
//             AllocLinkError::DuplicatedLinkName => Self {
//                 condition: AmqpError::NotAllowed.into(),
//                 description: Some("Link name is duplicated".to_string()),
//                 info: None,
//             },
//         }
//     }
// }
