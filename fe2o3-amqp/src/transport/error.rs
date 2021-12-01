use std::io;

use fe2o3_amqp_types::definitions::{AmqpError, ConnectionError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO Error {0:?}")]
    IoError(#[from] io::Error),

    #[error("AMQP error {0:?}")]
    AmqpError(#[from] AmqpError),

    #[error("Connection error {0:?}")]
    ConnectionError(#[from] ConnectionError),
}

impl From<Error> for crate::error::EngineError {
    fn from(err: Error) -> Self {
        use crate::error::EngineError;

        match err {
            Error::IoError(err) => EngineError::Io(err),
            Error::AmqpError(err) => EngineError::AmqpError(err),
            Error::ConnectionError(err) => EngineError::ConnectionError(err)
        }
    }
}