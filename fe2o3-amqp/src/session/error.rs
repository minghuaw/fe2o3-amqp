use std::io;

use fe2o3_amqp_types::definitions::{AmqpError, SessionError};
use tokio::task::JoinError;

use crate::connection::AllocSessionError;


#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("All channels have been allocated")]
    ChannelMaxExceeded,

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
            AllocSessionError::ChannelMaxExceeded => Self::ChannelMaxExceeded,
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