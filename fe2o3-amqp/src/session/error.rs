use std::io;

use fe2o3_amqp_types::definitions::{AmqpError, SessionError, self};
use tokio::task::JoinError;

use crate::connection::AllocSessionError;

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
    },

    #[error("Remote error {:?}", .0)]
    RemoteError(definitions::Error)
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

#[derive(Debug, thiserror::Error)]
pub enum AllocLinkError {
    #[error("Illegal session state")]
    IllegalState,

    #[error("Reached session handle max")]
    HandleMaxReached,

    #[error("Link name must be unique")]
    DuplicatedLinkName,
}

impl From<AllocSessionError> for Error {
    fn from(err: AllocSessionError) -> Self {
        match err {
            AllocSessionError::Io(e) => Self::Io(e),
            AllocSessionError::ChannelMaxReached => Self::ChannelMaxReached,
            AllocSessionError::IllegalState => Self::AmqpError {
                condition: AmqpError::IllegalState,
                description: None,
            },
        }
    }
}

// #[derive(Debug, thiserror::Error)]
// pub enum EndError {
//     #[error("Illegal state")]
//     IllegalState,
    
//     #[error("Remote session error: {:?}", .0)]
//     RemoteSessionError(definitions::Error),

//     // #[error("None")]
//     // None,
// }