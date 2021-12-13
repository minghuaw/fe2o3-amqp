use fe2o3_amqp_types::definitions::{AmqpError, LinkError};

use crate::session::AllocLinkError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Handle max reached")]
    HandleMaxReached,

    #[error("Link name must be unique")]
    DuplicatedLinkName,

    #[error("AMQP Error {:?}, {:?}", .condition, .description)]
    AmqpError {
        condition: AmqpError,
        // Option<String> takes the same amount of memory
        description: Option<String>,
    },

    #[error("Link Error {:?}, {:?}", .condition, .description)]
    LinkError {
        condition: LinkError,
        description: Option<String>,
    },
}

impl From<AllocLinkError> for Error {
    fn from(err: AllocLinkError) -> Self {
        match err {
            AllocLinkError::IllegalState => Self::AmqpError {
                condition: AmqpError::IllegalState,
                description: Some(String::from("Invalid session state")),
            },
            AllocLinkError::HandleMaxReached => Self::HandleMaxReached,
            AllocLinkError::DuplicatedLinkName => Self::DuplicatedLinkName,
        }
    }
}
