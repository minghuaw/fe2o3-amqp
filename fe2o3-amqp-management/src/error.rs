//! Error types for the management client.

use fe2o3_amqp::link::{
    DispositionError, ReceiverAttachError, RecvError, SendError, SenderAttachError,
};
use fe2o3_amqp_types::messaging::Outcome;

use crate::status::StatusCode;

/// An error that can occur when attaching the management client.
#[derive(Debug, thiserror::Error)]
pub enum AttachError {
    /// An error occurred when attaching the sender link.
    #[error(transparent)]
    Sender(#[from] SenderAttachError),

    /// An error occurred when attaching the receiver link.
    #[error(transparent)]
    Receiver(#[from] ReceiverAttachError),
}

/// Response status code is different from expected
#[derive(Debug)]
pub struct StatusError {
    /// Received status code
    pub code: StatusCode,
    /// Received status description
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

/// Error decoding from message. The received value is encoded in a different type than expected.
#[derive(Debug)]
pub struct InvalidType {
    /// Expected type
    pub expected: String,

    /// Actual type
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

/// StatusCode is not found
#[derive(Debug)]
pub struct StatusCodeNotFound {}

/// Error type for the management client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Correlation ID or Message ID is not found
    #[error("Correlation ID or Message ID is not found")]
    CorrelationIdAndMessageIdAreNone,

    /// StatusCode is not found
    #[error("StatusCode is nor found")]
    StatusCodeNotFound,

    /// Error with decoding from message
    #[error("Error decoding from message")]
    DecodeError(Option<InvalidType>),

    /// Status code is different from expected
    #[error(transparent)]
    Status(#[from] StatusError),

    /// Error with sending the request
    #[error(transparent)]
    Send(#[from] SendError),

    /// Request is not accepted
    #[error("Request is not accepted: {:?}", .0)]
    NotAccepted(Outcome),

    /// Error with receiving the response
    #[error(transparent)]
    Recv(#[from] RecvError),

    /// Error with accepting the response
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
