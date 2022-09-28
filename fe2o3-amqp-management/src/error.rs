use fe2o3_amqp::link::{DispositionError, ReceiverAttachError, RecvError, SenderAttachError, SendError};
use fe2o3_amqp_types::messaging::Outcome;

use crate::status::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum AttachError {
    #[error(transparent)]
    Sender(#[from] SenderAttachError),

    #[error(transparent)]
    Receiver(#[from] ReceiverAttachError),
}

#[derive(Debug)]
pub struct StatusError {
    pub code: StatusCode,
    pub description: Option<String>,
}

impl std::fmt::Display for StatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "StatusError {{code: {:?}, description: {:?} }}", self.code, self.description)
    }
}

impl std::error::Error for StatusError {}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    State(#[from] StatusError),

    #[error("Correlation ID or Message ID is not found")]
    CorrelationIdAndMessageIdAreNone,

    #[error("StatusCode is nor found")]
    StatusCodeNotFound,

    #[error("Error decoding from message")]
    DecodeError,

    #[error("Wrong status code {}", 0.0)]
    Status(StatusCode),

    #[error(transparent)]
    Send(#[from] SendError),

    #[error("Request is not accepted: {:?}", .0)]
    NotAccepted(Outcome),

    #[error(transparent)]
    Recv(#[from] RecvError),

    #[error(transparent)]
    Disposition(#[from] DispositionError),
}

impl From<Outcome> for Error {
    fn from(outcome: Outcome) -> Self {
        Self::NotAccepted(outcome)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
