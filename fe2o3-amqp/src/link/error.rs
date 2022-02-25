use std::{fmt, io};

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, ErrorCondition, LinkError},
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

impl<L> fmt::Display for DetachError<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DetachError")
            .field("link", &"")
            .field("is_closed_by_remote", &self.is_closed_by_remote)
            .field("error", &self.error)
            .finish()
    }
}

impl<L: fmt::Debug> std::error::Error for DetachError<L> {}

impl<L> TryFrom<Error> for DetachError<L> {
    type Error = Error;

    fn try_from(value: Error) -> Result<Self, Self::Error> {
        match value {
            Error::Io(error) => {
                let error = definitions::Error::new(
                    AmqpError::IllegalState,
                    Some(format!("{:?}", error)),
                    None
                );
                let err = Self {
                    link: None,
                    is_closed_by_remote: false,
                    error: Some(error)
                };
                Ok(err)
            },
            Error::LocalError(error) => {
                let err = Self {
                    link: None,
                    is_closed_by_remote: false,
                    error: Some(error)
                };
                Ok(err)
            },
            Error::Detached(err) => {
                let error = DetachError {
                    link: None,
                    is_closed_by_remote: err.is_closed_by_remote,
                    error: err.error
                };
                Ok(error)
            },
            Error::ParseError
            | Error::Rejected(_)
            | Error::Released(_)
            | Error::Modified(_) => Err(value),
        }
    }
}

impl<L> TryFrom<(L, Error)> for DetachError<L> {
    type Error = Error;

    fn try_from((link, value): (L, Error)) -> Result<Self, Self::Error> {
        match value {
            Error::Io(error) => {
                let error = definitions::Error::new(
                    AmqpError::IllegalState,
                    Some(format!("{:?}", error)),
                    None
                );
                let err = Self {
                    link: Some(link),
                    is_closed_by_remote: false,
                    error: Some(error)
                };
                Ok(err)
            },
            Error::LocalError(error) => {
                let err = Self {
                    link: Some(link),
                    is_closed_by_remote: false,
                    error: Some(error)
                };
                Ok(err)
            },
            Error::Detached(err) => {
                let error = DetachError {
                    link: Some(link),
                    is_closed_by_remote: err.is_closed_by_remote,
                    error: err.error
                };
                Ok(error)
            },
            Error::ParseError
            | Error::Rejected(_)
            | Error::Released(_)
            | Error::Modified(_) => Err(value),
        }
    }
}

/// TODO: Simplify the error structures
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("Parse error")]
    ParseError,
    
    #[error("Link is detached {:?}", .0)]
    Detached(DetachError<()>),
    
    #[error("Local error: {:?}", .0)]
    LocalError(definitions::Error),

    #[error("Outcome Rejected: {:?}", .0)]
    Rejected(Rejected),

    #[error("Outsome Released: {:?}", .0)]
    Released(Released),

    #[error("Outcome Modified: {:?}", .0)]
    Modified(Modified),
}

impl Error {
    // May want to have different handling of SendError
    pub(crate) fn error_sending_to_session() -> Self {
        Self::LocalError(definitions::Error::new(
            AmqpError::IllegalState,
            Some("Failed to send to sesssion".to_string()),
            None
        ))
    }

    pub(crate) fn amqp_error(condition: AmqpError, description: impl Into<Option<String>>) -> Self {
        Self::LocalError(definitions::Error::new(condition, description, None))
    }
}

impl From<AmqpError> for Error {
    fn from(err: AmqpError) -> Self {
        Self::LocalError(definitions::Error::new(
            err,
            None,
            None
        ))
    }
}

impl From<LinkError> for Error {
    fn from(err: LinkError) -> Self {
        Self::LocalError(definitions::Error::new(
            err,
            None,
            None
        ))
    }
}

impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(_: mpsc::error::SendError<T>) -> Self {
        Self::LocalError(definitions::Error::new(
            AmqpError::IllegalState,
            Some("Failed to send to sesssion".to_string()),
            None
        ))
    }
}


impl From<serde_amqp::Error> for Error {
    fn from(_: serde_amqp::Error) -> Self {
        Self::ParseError
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

#[derive(Debug, thiserror::Error)]
pub enum AttachError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("Illegal session state")]
    IllegalSessionState,

    #[error("Handle max reached")]
    HandleMaxReached,

    #[error("Link name must be unique")]
    DuplicatedLinkName,

    #[error("Local error: {:?}", .0)]
    LocalError(definitions::Error),
}

impl From<AllocLinkError> for AttachError {
    fn from(error: AllocLinkError) -> Self {
        match error {
            AllocLinkError::IllegalState => Self::IllegalSessionState,
            AllocLinkError::HandleMaxReached => Self::HandleMaxReached,
            AllocLinkError::DuplicatedLinkName => Self::DuplicatedLinkName,
        }
    }
}

impl TryFrom<Error> for AttachError {
    type Error = Error;

    fn try_from(value: Error) -> Result<Self, Self::Error> {
        match value {
            Error::Io(error) => Ok(AttachError::Io(error)),
            Error::LocalError(error) => Ok(AttachError::LocalError(error)),
            Error::ParseError
            | Error::Rejected(_)
            | Error::Released(_)
            | Error::Modified(_)
            | Error::Detached(_) => Err(value),
        }
    }
}