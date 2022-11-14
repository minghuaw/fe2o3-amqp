use fe2o3_amqp::link::{
    DispositionError, ReceiverAttachError, RecvError, SendError, SenderAttachError,
};
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
        writeln!(
            f,
            "StatusError {{code: {:?}, description: {:?} }}",
            self.code, self.description
        )
    }
}

impl std::error::Error for StatusError {}

#[derive(Debug)]
pub struct InvalidType {
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Display for InvalidType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "InvalidType {{expected: {:?}, actual: {:?} }}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for InvalidType {}

pub struct StatusCodeNotFound {}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Correlation ID or Message ID is not found")]
    CorrelationIdAndMessageIdAreNone,

    #[error("StatusCode is nor found")]
    StatusCodeNotFound,

    #[error("Error decoding from message")]
    DecodeError(Option<InvalidType>),

    #[error(transparent)]
    Status(#[from] StatusError),

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

impl From<InvalidType> for Error {
    fn from(invalid_type: InvalidType) -> Self {
        Self::DecodeError(Some(invalid_type))
    }
}

impl From<StatusCodeNotFound> for Error {
    fn from(_: StatusCodeNotFound) -> Self {
        Self::StatusCodeNotFound
    }
}

pub type Result<T> = std::result::Result<T, Error>;
