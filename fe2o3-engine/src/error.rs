use std::time::Duration;

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
