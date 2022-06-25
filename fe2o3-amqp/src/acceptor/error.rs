//! Implements errors for the acceptors

use crate::link::{SenderAttachError, ReceiverAttachError};

/// Error accepting incoming attach
#[derive(Debug, thiserror::Error)]
pub enum AcceptorAttachError {
    /// Session stopped
    #[error("Session has stopped")]
    IllegalSessionState,

    /// Local sender is unable to accept incoming attach from remote receiver
    #[error("Local sender is unable to accept incoming attach from remote receiver")]
    LocalSender(SenderAttachError),

    /// Local receiver is unable to accept incoming attach from remote sender
    #[error("Local receiver is unable to accept incoming attach from remote sender")]
    LocalReceiver(ReceiverAttachError),
}

impl From<SenderAttachError> for AcceptorAttachError {
    fn from(value: SenderAttachError) -> Self {
        if let SenderAttachError::IllegalSessionState = value {
            Self::IllegalSessionState
        } else {
            Self::LocalSender(value)
        }
    }
}

impl From<ReceiverAttachError> for AcceptorAttachError {
    fn from(value: ReceiverAttachError) -> Self {
        if let ReceiverAttachError::IllegalSessionState = value {
            Self::IllegalSessionState
        } else {
            Self::LocalReceiver(value)
        }
    }
}