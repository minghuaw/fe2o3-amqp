use fe2o3_amqp_types::{
    definitions,
    messaging::{Modified, Rejected, Released},
};

use crate::link::{self, AttachError, DetachError};

/// An error associated declaring a transaction
#[derive(Debug, thiserror::Error)]
pub enum DeclareError {
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

impl From<AttachError> for DeclareError {
    fn from(error: AttachError) -> Self {
        match error {
            AttachError::IllegalSessionState => Self::IllegalSessionState,
            AttachError::HandleMaxReached => Self::HandleMaxReached,
            AttachError::DuplicatedLinkName => Self::DuplicatedLinkName,
            AttachError::SourceIsNone => Self::SourceIsNone,
            AttachError::TargetIsNone => Self::TargetIsNone,
            AttachError::Local(e) => Self::Local(e),
        }
    }
}

impl From<link::SendError> for DeclareError {
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

/// The transaction manager was unable to allocate new transaction IDs
#[derive(Debug)]
pub enum TransactionManagerError {
    /// The transaction manager failed to allocate a new transaction ID
    AllocateTxnIdFailed,
}

/// Error when discharging a transaction
#[derive(Debug, thiserror::Error)]
pub enum DischargeError {
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

    /// The transaction has already discharged
    #[error("The transaction has already discharged")]
    AlreadyDischarged,
}

impl<E> From<E> for DischargeError where E: Into<link::SendError> {
    fn from(err: E) -> Self {
        match err.into() {
            link::SendError::Local(error) => Self::Local(error),
            link::SendError::Detached(error) => Self::Detached(error),
            link::SendError::Rejected(error) => Self::Rejected(error),
            link::SendError::Released(error) => Self::Released(error),
            link::SendError::Modified(error) => Self::Modified(error),
        }
    }
}