//! Implements errors for the acceptors

use std::sync::OnceLock;

use crate::link::{ReceiverAttachError, SenderAttachError, SessionOutcome, SessionStopped};

/// Error accepting incoming attach
#[derive(Debug, thiserror::Error)]
pub enum AcceptorAttachError {
    /// The session (or its connection) stopped
    #[error("The session stopped before the link was attached: {:?}", .0)]
    SessionStopped(SessionStopped),

    /// Local sender is unable to accept incoming attach from remote receiver
    #[error("Local sender is unable to accept incoming attach from remote receiver")]
    LocalSender(SenderAttachError),

    /// Local receiver is unable to accept incoming attach from remote sender
    #[error("Local receiver is unable to accept incoming attach from remote sender")]
    LocalReceiver(ReceiverAttachError),
}

/// The [`AcceptorAttachError`] for an accept that failed because the session
/// (or its connection) stopped; `SessionStopped(Ended)` when no stop reason
/// was recorded (defensive).
pub(crate) fn acceptor_attach_error_from_stop_reason(
    cell: &OnceLock<SessionStopped>,
) -> AcceptorAttachError {
    match cell.get() {
        Some(reason) => AcceptorAttachError::SessionStopped(reason.clone()),
        None => {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "accept: session stop reason not recorded; reporting SessionStopped(Ended)"
            );
            #[cfg(feature = "log")]
            log::warn!("accept: session stop reason not recorded; reporting SessionStopped(Ended)");
            AcceptorAttachError::SessionStopped(SessionStopped::Outcome(SessionOutcome::Ended))
        }
    }
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
