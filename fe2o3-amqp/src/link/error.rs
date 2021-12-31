use std::convert::Infallible;

use fe2o3_amqp_types::{definitions::{self, AmqpError, ErrorCondition, LinkError}, messaging::{Rejected, Released, Modified}};
use tokio::sync::mpsc;

use crate::session::AllocLinkError;

#[derive(Debug)]
pub struct DetachError {
    pub(crate) is_closed_by_remote: bool,
    pub(crate) error: Option<definitions::Error>,
}

impl DetachError {
    pub fn is_closed_by_remote(&self) -> bool {
        self.is_closed_by_remote
    }

    pub fn error_condition(&self) -> Option<&ErrorCondition> {
        match &self.error {
            Some(e) => Some(&e.condition),
            None => None,
        }
    }

    pub fn into_error(self) -> Option<definitions::Error> {
        self.error
    }
}

impl<T> From<T> for DetachError
where
    T: Into<definitions::Error>,
{
    fn from(err: T) -> Self {
        Self {
            is_closed_by_remote: false,
            error: Some(err.into()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Handle max reached")]
    HandleMaxReached,

    #[error("Link name must be unique")]
    DuplicatedLinkName,

    #[error("Parse error")]
    ParseError,

    #[error("Outcome Rejected: {:?}", .0)]
    Rejected(Rejected),

    #[error("Outsome Released: {:?}", .0)]
    Released(Released),

    #[error("Outcome Modified: {:?}", .0)]
    Modified(Modified),

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

impl From<AmqpError> for Error {
    fn from(err: AmqpError) -> Self {
        Self::AmqpError {
            condition: err,
            description: None,
        }
    }
}

impl From<LinkError> for Error {
    fn from(err: LinkError) -> Self {
        Self::LinkError {
            condition: err,
            description: None,
        }
    }
}

impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(_: mpsc::error::SendError<T>) -> Self {
        Self::AmqpError {
            condition: AmqpError::IllegalState,
            description: Some("Failed to send to sesssion".to_string()),
        }
    }
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

impl From<serde_amqp::Error> for Error {
    fn from(_: serde_amqp::Error) -> Self {
        Self::ParseError
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        Self::AmqpError {
            condition: AmqpError::InternalError,
            description: Some("Infallible".to_string()),
        }
    }
}
