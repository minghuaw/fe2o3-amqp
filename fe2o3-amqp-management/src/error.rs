use fe2o3_amqp::link::{DispositionError, ReceiverAttachError, RecvError, SenderAttachError};

use crate::status::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum AttachError {
    #[error(transparent)]
    Sender(#[from] SenderAttachError),

    #[error(transparent)]
    Receiver(#[from] ReceiverAttachError),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Correlation ID or Message ID is not found")]
    CorrelationIdAndMessageIdAreNone,

    #[error("StatusCode is nor found")]
    StatusCodeNotFound,

    #[error("Error decoding from message")]
    DecodeError,

    #[error("Wrong status code {}", 0.0)]
    Status(StatusCode),

    #[error(transparent)]
    Recv(#[from] RecvError),

    #[error(transparent)]
    Disposition(#[from] DispositionError),
}

pub type Result<T> = std::result::Result<T, Error>;
