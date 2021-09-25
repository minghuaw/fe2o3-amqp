use fe2o3_types::definitions::{AmqpError, ConnectionError};
use thiserror::Error;

use crate::transport::connection::ConnectionState;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("IO Error: {0:?}")]
    Io(#[from] std::io::Error),

    #[error("Url Error: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("Connection/transport is not connected")]
    IsClosed,

    #[error("Parse Error: {0}")]
    ParseError(#[from] fe2o3_amqp::Error),

    #[error("Unexpected Protocol ID {0:?}")]
    UnexpectedProtocolId(u8),

    #[error("Unexpected Protocol Header. Found {0:?}")]
    UnexpectedProtocolHeader([u8; 8]),

    #[error("Maximum frame size is exceeded")]
    MaxFrameSizeExceeded,

    #[error("The frame is malformed")]
    MalformedFrame,

    #[error("Invalid Connection State {0:?}")]
    UnexpectedConnectionState(ConnectionState),

    #[error("AMQP Error: {0:?}")]
    AmqpError(#[from] AmqpError),

    #[error("Connection Error {0:?}")]
    ConnectionError(#[from] ConnectionError),

    #[error("Connection error idle timeout")]
    IdleTimeout,

    #[error("{0}")]
    Message(&'static str),
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for EngineError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Message("SendError")
    }
}

impl EngineError {
    /// The peer sent a frame that is not permitted in the current state.
    pub fn illegal_state() -> Self {
        Self::AmqpError(AmqpError::IllegalState)
    }

    pub fn not_found() -> Self {
        Self::AmqpError(AmqpError::NotFound)
    }

    pub fn invalid_field() -> Self {
        Self::AmqpError(AmqpError::InvalidField)
    }

    pub fn not_allowed() -> Self {
        Self::AmqpError(AmqpError::NotAllowed)
    }
}
