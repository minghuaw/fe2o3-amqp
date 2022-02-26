use std::{fmt};

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, ErrorCondition, LinkError},
    messaging::{Modified, Rejected, Released},
};
use tokio::sync::mpsc;

use crate::session::AllocLinkError;

/// 
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

    pub(crate) fn empty() -> Self {
        Self {
            link: None,
            is_closed_by_remote: false,
            error: None
        }
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
            Error::Local(error) => {
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
            Error::Local(error) => {
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
    #[error("Parse error")]
    ParseError,
    
    #[error("Local error: {:?}", .0)]
    Local(definitions::Error),

    #[error("Link is detached {:?}", .0)]
    Detached(DetachError<()>),
    
    #[error("Outcome Rejected: {:?}", .0)]
    Rejected(Rejected),

    #[error("Outsome Released: {:?}", .0)]
    Released(Released),

    #[error("Outcome Modified: {:?}", .0)]
    Modified(Modified),
}

impl Error {
    // May want to have different handling of SendError
    pub(crate) fn sending_to_session() -> Self {
        Self::Local(definitions::Error::new(
            AmqpError::IllegalState,
            Some("Failed to send to sesssion".to_string()),
            None
        ))
    }

    pub(crate) fn expecting_frame(frame_ident: impl Into<String>) -> Self {
        Self::Local(definitions::Error::new(
            AmqpError::IllegalState,
            Some(format!("Expecting {}", frame_ident.into())),
            None
        ))
    }

    pub(crate) fn not_attached() -> Self {
        Self::Local(definitions::Error::new(
            AmqpError::IllegalState,
            Some("Link is not attached".to_string()),
            None
        ))
    }
}

impl From<AmqpError> for Error {
    fn from(err: AmqpError) -> Self {
        Self::Local(definitions::Error::new(
            err,
            None,
            None
        ))
    }
}

impl From<LinkError> for Error {
    fn from(err: LinkError) -> Self {
        Self::Local(definitions::Error::new(
            err,
            None,
            None
        ))
    }
}

impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(_: mpsc::error::SendError<T>) -> Self {
        Self::Local(definitions::Error::new(
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
            Error::Local(error) => Ok(AttachError::LocalError(error)),
            Error::ParseError
            | Error::Rejected(_)
            | Error::Released(_)
            | Error::Modified(_)
            | Error::Detached(_) => Err(value),
        }
    }
}