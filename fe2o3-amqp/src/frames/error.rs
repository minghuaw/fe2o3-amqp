use std::io;

/// Errors associated with frame encoder and decoder
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// IO error
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    /// AMQP error: decode error
    #[error("Decode Error")]
    DecodeError(String),

    /// AMQP error: not implemented
    #[error("AmqpError: NotImplemented")]
    NotImplemented,
}

impl From<serde_amqp::Error> for Error {
    fn from(err: serde_amqp::Error) -> Self {
        match err {
            serde_amqp::Error::Io(e) => Self::Io(e),
            other => Self::DecodeError(other.to_string()),
        }
    }
}
