use std::fmt;

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, ErrorCondition, LinkError},
    messaging::{Modified, Rejected, Released},
};
use tokio::sync::mpsc;

use crate::session::AllocLinkError;

/// Error associated with detaching a link
#[derive(Debug)]
pub struct DetachError {
    // /// The link which encountered error while detaching
    // pub link: Option<L>,
    /// Whether the remote is closing
    pub is_closed_by_remote: bool,
    /// The error associated with detachment
    pub error: Option<definitions::Error>,
}

impl DetachError {
    pub(crate) fn new(
        is_closed_by_remote: bool,
        error: Option<definitions::Error>,
    ) -> Self {
        Self {
            is_closed_by_remote,
            error,
        }
    }

    /// Whether the remote decided to close
    pub fn is_closed_by_remote(&self) -> bool {
        self.is_closed_by_remote
    }

    /// The error condition
    pub fn error_condition(&self) -> Option<&ErrorCondition> {
        match &self.error {
            Some(e) => Some(&e.condition),
            None => None,
        }
    }

    /// Convert into the inner error
    pub fn into_error(self) -> Option<definitions::Error> {
        self.error
    }
}

impl fmt::Display for DetachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DetachError")
            .field("is_closed_by_remote", &self.is_closed_by_remote)
            .field("error", &self.error)
            .finish()
    }
}

impl std::error::Error for DetachError {}

impl TryFrom<Error> for DetachError {
    type Error = Error;

    fn try_from(value: Error) -> Result<Self, Self::Error> {
        match value {
            Error::Local(error) => {
                let err = Self {
                    is_closed_by_remote: false,
                    error: Some(error),
                };
                Ok(err)
            }
            Error::Detached(err) => {
                let error = DetachError {
                    is_closed_by_remote: err.is_closed_by_remote,
                    error: err.error,
                };
                Ok(error)
            }
            Error::Rejected(_) | Error::Released(_) | Error::Modified(_) => Err(value),
        }
    }
}

/// Error associated with normal operations on a link
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A local error
    #[error("Local error: {:?}", .0)]
    Local(definitions::Error),

    /// The remote peer detached with error
    #[error("Link is detached {:?}", .0)]
    Detached(DetachError),

    /// The message was rejected
    #[error("Outcome Rejected: {:?}", .0)]
    Rejected(Rejected),

    /// The message was released
    #[error("Outsome Released: {:?}", .0)]
    Released(Released),

    /// The message was modified
    #[error("Outcome Modified: {:?}", .0)]
    Modified(Modified),
}

impl Error {
    // May want to have different handling of SendError
    pub(crate) fn sending_to_session() -> Self {
        Self::Local(definitions::Error::new(
            AmqpError::IllegalState,
            Some("Failed to send to sesssion".to_string()),
            None,
        ))
    }

    pub(crate) fn expecting_frame(frame_ident: impl Into<String>) -> Self {
        Self::Local(definitions::Error::new(
            AmqpError::IllegalState,
            Some(format!("Expecting {}", frame_ident.into())),
            None,
        ))
    }

    pub(crate) fn not_attached() -> Self {
        Self::Local(definitions::Error::new(
            AmqpError::IllegalState,
            Some("Link is not attached".to_string()),
            None,
        ))
    }
}

impl From<AmqpError> for Error {
    fn from(err: AmqpError) -> Self {
        Self::Local(definitions::Error::new(err, None, None))
    }
}

impl From<LinkError> for Error {
    fn from(err: LinkError) -> Self {
        Self::Local(definitions::Error::new(err, None, None))
    }
}

impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(_: mpsc::error::SendError<T>) -> Self {
        Self::Local(definitions::Error::new(
            AmqpError::IllegalState,
            Some("Failed to send to sesssion".to_string()),
            None,
        ))
    }
}

impl From<serde_amqp::Error> for Error {
    fn from(err: serde_amqp::Error) -> Self {
        Self::Local(definitions::Error::new(
            AmqpError::DecodeError,
            Some(format!("{:?}", err)),
            None,
        ))
    }
}

pub(crate) fn detach_error_expecting_frame() -> DetachError {
    let error = definitions::Error::new(
        AmqpError::IllegalState,
        Some("Expecting remote detach frame".to_string()),
        None,
    );

    DetachError {
        is_closed_by_remote: false,
        error: Some(error),
    }
}

/// Error associated with attaching a link
#[derive(Debug, thiserror::Error)]
pub enum AttachError {
    /// Session is in an illegal state
    #[error("Illegal session state")]
    IllegalSessionState,

    /// Session's max number of handle has reached
    #[error("Handle max reached")]
    HandleMaxReached,

    /// Link name is duplicated
    #[error("Link name must be unique")]
    DuplicatedLinkName,

    // /// Initial delivery count field MUST NOT be null if role is sender, and it is ignored if the role is receiver.
    // #[error("Initial delivery count MUST NOT be null if role is sender,")]
    // InitialDeliveryCountIsNull,
    /// Source field in Attach is Null
    #[error("Source is None")]
    SourceIsNone,

    /// Target field in Attach is Null
    #[error("Target is None")]
    TargetIsNone,

    /// The desired mode is not supported
    #[error("Desired receiver settle mode is not supported")]
    ReceiverSettleModeNotSupported,

    /// The desired mode is not supported
    #[error("Desired sender settle mode is not supported")]
    SenderSettleModeNotSupported,

    /// A local error
    #[error("Local error: {:?}", .0)]
    Local(definitions::Error),
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
            Error::Local(error) => Ok(AttachError::Local(error)),
            Error::Rejected(_) | Error::Released(_) | Error::Modified(_) | Error::Detached(_) => {
                Err(value)
            }
        }
    }
}

impl AttachError {
    pub(crate) fn illegal_state(description: impl Into<Option<String>>) -> Self {
        Self::Local(definitions::Error::new(
            AmqpError::IllegalState,
            description.into(),
            None,
        ))
    }
}
