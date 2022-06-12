use fe2o3_amqp_types::{
    definitions,
    messaging::{Modified, Rejected, Released},
};

use crate::link::{self, AttachError, DetachError};

use super::{Controller, Undeclared};

/// Error kind of a declare error
#[derive(Debug, thiserror::Error)]
pub enum DeclareErrorKind {
    /// Session is in an illegal state
    #[error("Illegal session state")]
    IllegalSessionState,

    /// Session's max number of handle has reached
    #[error("Handle max reached")]
    HandleMaxReached,

    /// Link name is duplicated
    #[error("Link name must be unique")]
    DuplicatedLinkName,

    /// Initial delivery count field MUST NOT be null if role is sender, and it is ignored if the role is receiver.
    /// #[error("Initial delivery count MUST NOT be null if role is sender,")]
    /// InitialDeliveryCountIsNull,
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

/// An error associated declaring a transaction
#[derive(Debug)]
pub struct DeclareError {
    /// The controller used for declaration
    pub controller: Option<Controller<Undeclared>>,

    /// Error associated with the declaration
    pub kind: DeclareErrorKind,
}

impl From<AttachError> for DeclareErrorKind {
    fn from(error: AttachError) -> Self {
        match error {
            AttachError::IllegalSessionState => Self::IllegalSessionState,
            AttachError::HandleMaxReached => Self::HandleMaxReached,
            AttachError::DuplicatedLinkName => Self::DuplicatedLinkName,
            AttachError::SourceIsNone => Self::SourceIsNone,
            AttachError::TargetIsNone => Self::TargetIsNone,
            AttachError::ReceiverSettleModeNotSupported => Self::ReceiverSettleModeNotSupported,
            AttachError::SenderSettleModeNotSupported => Self::SenderSettleModeNotSupported,
            AttachError::Local(e) => Self::Local(e),
        }
    }
}

impl From<link::SendError> for DeclareErrorKind {
    fn from(error: link::SendError) -> Self {
        match error {
            link::SendError::Local(e) => Self::Local(e),
            link::SendError::Detached(e) => Self::Detached(e),
            link::SendError::Rejected(e) => Self::Rejected(e),
            link::SendError::Released(e) => Self::Released(e),
            link::SendError::Modified(e) => Self::Modified(e),
        }
    }
}

impl<T> From<(Controller<Undeclared>, T)> for DeclareError
where
    T: Into<DeclareErrorKind>,
{
    fn from((controller, err): (Controller<Undeclared>, T)) -> Self {
        Self {
            controller: Some(controller),
            kind: err.into(),
        }
    }
}

impl<T> From<T> for DeclareError
where
    T: Into<DeclareErrorKind>,
{
    fn from(kind: T) -> Self {
        Self {
            controller: None,
            kind: kind.into(),
        }
    }
}
