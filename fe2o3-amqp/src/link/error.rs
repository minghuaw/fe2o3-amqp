use std::convert::Infallible;

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, ConnectionError, ErrorCondition, LinkError},
    messaging::{Modified, Rejected, Released},
};
use tokio::sync::mpsc;

use crate::session::AllocLinkError;

#[derive(Debug)]
pub struct DetachError<L> {
    pub link: Option<L>,
    pub(crate) is_closed_by_remote: bool,
    pub(crate) error: Option<definitions::Error>,
}

impl<L> DetachError<L> {
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

// impl<T> From<T> for DetachError
// where
//     T: Into<definitions::Error>,
// {
//     fn from(err: T) -> Self {
//         Self {
//             is_closed_by_remote: false,
//             error: Some(err.into()),
//         }
//     }
// }

/// TODO: Simplify the error structures
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

    #[error("Link is detached {:?}", .0)]
    Detached(DetachError<()>),

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

impl Error {
    pub(crate) fn error_sending_to_session() -> Self {
        Self::AmqpError {
            condition: AmqpError::IllegalState,
            description: Some("Failed to send to sesssion".to_string()),
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

impl From<AllocLinkError> for definitions::Error {
    fn from(err: AllocLinkError) -> Self {
        match err {
            AllocLinkError::IllegalState => Self {
                condition: AmqpError::IllegalState.into(),
                description: None,
                info: None,
            },
            AllocLinkError::HandleMaxReached => Self {
                condition: ConnectionError::FramingError.into(),
                description: Some("Handle max has been reached".to_string()),
                info: None,
            },
            AllocLinkError::DuplicatedLinkName => Self {
                condition: AmqpError::NotAllowed.into(),
                description: Some("Link name is duplicated".to_string()),
                info: None,
            },
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

pub(crate) fn detach_error_expecting_frame<L>(link: L) -> DetachError<L> {
    let error = definitions::Error::new(
        AmqpError::IllegalState,
        Some("Expecting remote detach frame".to_string()),
        None,
    );

    DetachError {
        link: Some(link),
        is_closed_by_remote: false,
        error: Some(error),
    }
}

pub(crate) fn map_send_detach_error<L>(err: impl Into<Error>, link: L) -> DetachError<L> {
    let (condition, description): (ErrorCondition, _) = match err.into() {
        Error::AmqpError {
            condition,
            description,
        } => (condition.into(), description),
        Error::LinkError {
            condition,
            description,
        } => (condition.into(), description),
        Error::Detached(e) => {
            return DetachError {
                link: None,
                is_closed_by_remote: e.is_closed_by_remote,
                error: e.error,
            }
        }
        Error::HandleMaxReached
        | Error::DuplicatedLinkName
        | Error::ParseError
        | Error::Rejected(_)
        | Error::Released(_)
        | Error::Modified(_) => unreachable!(),
    };
    DetachError {
        link: Some(link),
        is_closed_by_remote: false,
        error: Some(definitions::Error::new(condition, description, None)),
    }
}
