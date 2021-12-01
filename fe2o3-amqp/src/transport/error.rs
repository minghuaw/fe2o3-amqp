use std::io;

use fe2o3_amqp_types::definitions::{AmqpError, ConnectionError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    #[error("Idle timeout")]
    IdleTimeout,

    #[error("AMQP error {:?}, {:?}", .condition, .description)]
    AmqpError{
        condition: AmqpError,
        description: Option<String>,
    },

    #[error("Connection error {:?}, {:?}", .condition, .description)]
    ConnectionError {
        condition: ConnectionError,
        description: Option<String>
    }
}

impl Error {
    pub fn amqp_error(condition: impl Into<AmqpError>, description: impl Into<Option<String>>) -> Self {
        Self::AmqpError {
            condition: condition.into(),
            description: description.into()
        }
    }

    pub fn connection_error(condition: impl Into<ConnectionError>, description: impl Into<Option<String>>) -> Self {
        Self::ConnectionError {
            condition: condition.into(),
            description: description.into()
        }
    }
}

impl From<serde_amqp::Error> for Error {
    fn from(err: serde_amqp::Error) -> Self {
        match err {
            serde_amqp::Error::Io(e) => Self::Io(e),
            e @ _ => {
                let description = e.to_string();
                Self::AmqpError {
                    condition: AmqpError::DecodeError,
                    description: Some(description)
                }
            }
        }
    }
}

impl From<AmqpError> for Error {
    fn from(err: AmqpError) -> Self {
        Self::AmqpError {
            condition: err,
            description: None
        }
    }
}

impl From<ConnectionError> for Error {
    fn from(err: ConnectionError) -> Self {
        Self::ConnectionError {
            condition: err,
            description: None
        }
    }
}

impl From<Error> for crate::error::EngineError {
    fn from(err: Error) -> Self {
        use crate::error::EngineError;

        match err {
            Error::Io(err) => EngineError::Io(err),
            Error::IdleTimeout => EngineError::IdleTimeout,
            Error::AmqpError{condition, description} => EngineError::AmqpError(condition),
            Error::ConnectionError{condition, description} => EngineError::ConnectionError(condition)
        }
    }
}
