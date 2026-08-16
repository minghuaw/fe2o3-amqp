//! Implements errors for the acceptors

use crate::link::{ReceiverAttachError, SenderAttachError, SessionStopReason};

/// Error accepting incoming attach
#[derive(Debug, thiserror::Error)]
pub enum AcceptorAttachError {
    /// The session (or its connection) stopped
    #[error("The session stopped before the link was attached: {:?}", .0)]
    SessionStopped(SessionStopReason),

    /// Local sender is unable to accept incoming attach from remote receiver
    #[error("Local sender is unable to accept incoming attach from remote receiver")]
    LocalSender(SenderAttachError),

    /// Local receiver is unable to accept incoming attach from remote sender
    #[error("Local receiver is unable to accept incoming attach from remote sender")]
    LocalReceiver(ReceiverAttachError),
}

impl From<SenderAttachError> for AcceptorAttachError {
    fn from(value: SenderAttachError) -> Self {
        Self::LocalSender(value)
    }
}

impl From<ReceiverAttachError> for AcceptorAttachError {
    fn from(value: ReceiverAttachError) -> Self {
        Self::LocalReceiver(value)
    }
}
