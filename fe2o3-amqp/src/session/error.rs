use std::io;

use fe2o3_amqp_types::definitions::{AmqpError, SessionError};
use tokio::task::JoinError;

use crate::{connection::AllocSessionError, error::EngineError};


#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("All channels have been allocated")]
    ChannelMaxReached,

    #[error(transparent)]
    JoinError(#[from] JoinError),

    #[error("AMQP error {:?}, {:?}", .condition, .description)]
    AmqpError {
        condition: AmqpError,
        description: Option<String>,
    },

    #[error("Session error {:?}, {:?}", .condition, .description)]
    SessionError {
        condition: SessionError,
        description: Option<String>,
    }
}

impl From<AllocSessionError> for Error {
    fn from(err: AllocSessionError) -> Self {
        match err {
            AllocSessionError::Io(e) => Self::Io(e),
            AllocSessionError::ChannelMaxReached => Self::ChannelMaxReached,
            AllocSessionError::IllegalState => Self::AmqpError {
                condition: AmqpError::IllegalState,
                description: None
            },
        }
    }
}

impl From<AmqpError> for Error {
    fn from(err: AmqpError) -> Self {
        Self::AmqpError {
            condition: err,
            description: None,
        }
    }
}

impl From<SessionError> for Error {
    fn from(err: SessionError) -> Self {
        Self::SessionError {
            condition: err,
            description: None,
        }
    }
}

impl From<Error> for EngineError {
    fn from(err: Error) -> Self {
        match err {
            Error::Io(e) => EngineError::Io(e),
            Error::ChannelMaxReached => EngineError::Message("Channel max reached"),
            Error::JoinError(e) => EngineError::JoinError(e),
            Error::AmqpError {condition, description} => EngineError::AmqpError(condition),
            Error::SessionError {condition, description} => EngineError::SessionError(condition)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AllocLinkError {
    #[error("Illegal local state")]
    IllegalState,

    #[error("Reached session handle max")]
    HandleMaxReached,
}