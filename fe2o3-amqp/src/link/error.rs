use std::fmt;

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, ErrorCondition, LinkError, SessionError},
    messaging::{Modified, Rejected, Released},
};
use tokio::sync::{mpsc, oneshot, TryLockError};

use crate::session::AllocLinkError;

#[cfg(feature = "transaction")]
use fe2o3_amqp_types::transaction::TransactionId;

/// Error associated with detaching
#[derive(Debug, thiserror::Error)]
pub enum DetachError {
    /// ILlegal link state
    #[error("Illegal local state")]
    IllegalState,

    /// Session has dropped
    #[error("Session has dropped")]
    IllegalSessionState,

    /// Expecting a detach but found other frame
    #[error("Expecting a Detach")]
    NonDetachFrameReceived,

    /// Remote peer detached with error
    #[error("Remote detached with an error: {}", .0)]
    RemoteDetachedWithError(definitions::Error),

    /// Remote peer sent a closing detach when the local terminus sent a non-closing detach
    #[error("Link closed by remote")]
    ClosedByRemote,

    /// Remote peer sent a non-closing detach when the local terminus is sending a closing detach
    #[error("Link will be closed by local terminus")]
    DetachedByRemote,

    /// Remote peer closed the link with an error
    #[error("Remote peer closed the link with an error: {}", .0)]
    RemoteClosedWithError(definitions::Error)
}

impl From<DetachError> for Error {
    fn from(value: DetachError) -> Self {
        match value {
            DetachError::IllegalState => Self::IllegalState,
            DetachError::IllegalSessionState => Self::IllegalSessionState,
            DetachError::RemoteDetachedWithError(error) => Self::RemoteDetachedWithError(error),
            DetachError::ClosedByRemote => Self::RemoteClosed,
            DetachError::DetachedByRemote => Self::RemoteDetached,
            DetachError::RemoteClosedWithError(error) => Self::RemoteClosedWithError(error),
            DetachError::NonDetachFrameReceived => Self::ExpectImmediateDetach,
        }
    }
}

/// Error associated with sending a message
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    /// 
    #[error("Local error: {:?}", .0)]
    LinkError(#[from] SenderTransferError),

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

    /// Transactional state found on non-transactional delivery
    #[error("Transactional state found on non-transactional delivery")]
    IllegalDeliveryState,

    /// Error serializing message
    #[error("Error encoding message")]
    MessageEncodeError,
}

impl From<DetachError> for SendError {
    fn from(error: DetachError) -> Self {
        Self::Detached(error)
    }
}

/// Type alias for receiving error
pub type RecvError = Error;

/// Error associated with normal operations on a link
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// ILlegal link state
    #[error("Illegal local state")]
    IllegalState,

    /// Session has dropped
    #[error("Session has dropped")]
    IllegalSessionState,

    /// The delivery-id is not found in Transfer
    #[error("Delivery ID is not found in Transfer")]
    DeliveryIdIsNone,

    /// The delivery-tag is not found in Transfer
    #[error("Delivery tag is not found in Transfer")]
    DeliveryTagIsNone,

    /// Decoding Message failed
    #[error("Decoding Message failed")]
    MessageDecodeError,

    /// If the negotiated link value is first, then it is illegal to set this
    /// field to second.
    #[error("Negotiated value is first. Setting mode to second is illegal")]
    IllegalRcvSettleModeInTransfer,

    /// The link is expected to be detached immediately but didn't receive
    /// an incoming Detach frame
    #[error("Expecting an immediate detach")]
    ExpectImmediateDetach,

    /// Remote peer detached 
    #[error("Remote detached")]
    RemoteDetached,

    /// Remote peer detached with error
    #[error("Remote detached with an error: {}", .0)]
    RemoteDetachedWithError(definitions::Error),

    /// Remote peer closed 
    #[error("Remote closed")]
    RemoteClosed,

    /// Remote peer closed the link with an error
    #[error("Remote peer closed the link with an error: {}", .0)]
    RemoteClosedWithError(definitions::Error),

    /// The peer sent more message transfers than currently allowed on the link.
    #[error("The peer sent more message transfers than currently allowed on the link")]
    TransferLimitExceeded,

    /// Field is inconsisten in multi-frame delivery
    #[error("Field is inconsisten in multi-frame delivery")]
    InconsistentFieldInMultiFrameDelivery
}

impl From<oneshot::error::RecvError> for Error {
    fn from(_: oneshot::error::RecvError) -> Self {
        Self::IllegalSessionState
    }
}

/// Error associated with attaching a link
#[derive(Debug, thiserror::Error)]
pub enum AttachError {
    /// Session is in an illegal state
    #[error("Illegal session state")]
    IllegalSessionState,

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

    /// A local error
    #[error("Local error: {:?}", .0)]
    Local(definitions::Error),
}

impl From<AllocLinkError> for AttachError {
    fn from(error: AllocLinkError) -> Self {
        match error {
            AllocLinkError::IllegalSessionState => Self::IllegalSessionState,
            AllocLinkError::DuplicatedLinkName => Self::DuplicatedLinkName,
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

    pub(crate) fn not_implemented(description: impl Into<Option<String>>) -> Self {
        Self::Local(definitions::Error::new(
            AmqpError::NotImplemented,
            description,
            None,
        ))
    }

    pub(crate) fn not_allowed(description: impl Into<Option<String>>) -> Self {
        AttachError::Local(definitions::Error::new(
            AmqpError::NotAllowed,
            description,
            None,
        ))
    }
}

/// Error with the sender trying consume link credit
///
/// This is only used in
#[derive(Debug, thiserror::Error)]
pub enum SenderTryConsumeError {
    /// The sender is unable to acquire lock to inner state
    #[error("Try lock error")]
    TryLockError,

    /// There is not enough link credit
    #[error("Insufficient link credit")]
    InsufficientCredit,
}

impl From<TryLockError> for SenderTryConsumeError {
    fn from(_: TryLockError) -> Self {
        Self::TryLockError
    }
}

/// Errors associated with attaching a link as receiver
#[derive(Debug)]
pub enum ReceiverAttachError {
    // Errors that should end the session
    /// The associated session has dropped
    IllegalSessionState,
    
    /// Link name is already in use
    DuplicatedLinkName,

    /// Illegal link state
    IllegalState, 

    /// The local terminus is expecting an Attach from the remote peer
    NonAttachFrameReceived,

    /// The link is expected to be detached immediately but didn't receive
    /// an incoming Detach frame
    ExpectImmediateDetach,

    // Errors that should reject Attach
    /// Incoming Attach frame's Source field is None
    IncomingSourceIsNone,
    
    /// Incoming Attach frame's Target field is None
    IncomingTargetIsNone,

    /// The remote Attach contains a [`Coordinator`] in the Target
    CoordinatorIsNotImplemented,

    /// This MUST NOT be null if role is sender
    InitialDeliveryCountIsNone,

    /// When dynamic is set to true by the sending link endpoint, this field constitutes a request
    /// for the receiving peer to dynamically create a node at the target. In this case the address
    /// field MUST NOT be set.
    AddressIsSomeWhenDynamicIsTrue,

    /// If the dynamic field is not set to true this field MUST be left unset.
    DynamicNodePropertiesIsSomeWhenDynamicIsFalse,
}

impl From<AllocLinkError> for ReceiverAttachError {
    fn from(value: AllocLinkError) -> Self {
        match value {
            AllocLinkError::IllegalSessionState => Self::IllegalSessionState,
            AllocLinkError::DuplicatedLinkName => Self::DuplicatedLinkName,
        }
    }
}

impl<'a> TryFrom<&'a ReceiverAttachError> for definitions::Error {
    type Error = &'a ReceiverAttachError;

    fn try_from(value: &'a ReceiverAttachError) -> Result<Self, Self::Error> {
        let condition: ErrorCondition = match value {
            ReceiverAttachError::IllegalSessionState => AmqpError::IllegalState.into(),
            ReceiverAttachError::DuplicatedLinkName => SessionError::HandleInUse.into(),
            ReceiverAttachError::IllegalState => AmqpError::IllegalState.into(),
            ReceiverAttachError::NonAttachFrameReceived => AmqpError::NotAllowed.into(),
            ReceiverAttachError::ExpectImmediateDetach => AmqpError::NotAllowed.into(),
            ReceiverAttachError::IncomingSourceIsNone 
            | ReceiverAttachError::IncomingTargetIsNone => return Err(value),
            ReceiverAttachError::CoordinatorIsNotImplemented => AmqpError::NotImplemented.into(),
            ReceiverAttachError::InitialDeliveryCountIsNone => AmqpError::InvalidField.into(),
            ReceiverAttachError::AddressIsSomeWhenDynamicIsTrue => AmqpError::InvalidField.into(),
            ReceiverAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse => AmqpError::InvalidField.into(),
        };

        Ok(Self::new(condition, format!("{:?}", value), None))
    }
}

/// Errors associated with attaching a link as sender
#[derive(Debug)]
pub enum SenderAttachError {
    // Illegal session state

    /// Session stopped
    IllegalSessionState,

    /// Link name duplicated
    DuplicatedLinkName,

    /// Illegal link state
    IllegalState, 

    /// The local terminus is expecting an Attach from the remote peer
    NonAttachFrameReceived,

    /// The link is expected to be detached immediately but didn't receive
    /// an incoming Detach frame
    ExpectImmediateDetach,

    // Errors that should reject Attach
    /// Incoming Attach frame's Source field is None
    IncomingSourceIsNone,

    /// Incoming Attach frame's Target field is None
    IncomingTargetIsNone,

    /// The remote Attach contains a [`Coordinator`] in the Target
    CoordinatorIsNotImplemented,

    /// When set to true by the receiving link endpoint this field indicates creation of a
    /// dynamically created node. In this case the address field will contain the address of the
    /// created node.
    AddressIsNoneWhenDynamicIsTrue,

    /// If the dynamic field is not set to true this field MUST be left unset.
    DynamicNodePropertiesIsSomeWhenDynamicIsFalse,

    /// Desired TransactionCapabilities is not supported
    DesireTxnCapabilitiesNotSupported,
}

impl From<AllocLinkError> for SenderAttachError {
    fn from(value: AllocLinkError) -> Self {
        match value {
            AllocLinkError::IllegalSessionState => Self::IllegalSessionState,
            AllocLinkError::DuplicatedLinkName => Self::DuplicatedLinkName,
        }
    }
}

impl<'a> TryFrom<&'a SenderAttachError> for definitions::Error {
    type Error = &'a SenderAttachError;

    fn try_from(value: &'a SenderAttachError) -> Result<Self, Self::Error> {
        let condition: ErrorCondition = match value {
            SenderAttachError::IllegalSessionState => AmqpError::IllegalState.into(),
            SenderAttachError::DuplicatedLinkName => SessionError::HandleInUse.into(),
            SenderAttachError::IllegalState => AmqpError::IllegalState.into(),
            SenderAttachError::NonAttachFrameReceived => AmqpError::NotAllowed.into(),
            SenderAttachError::ExpectImmediateDetach => AmqpError::NotAllowed.into(),
            SenderAttachError::CoordinatorIsNotImplemented => AmqpError::NotImplemented.into(),
            SenderAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse => AmqpError::InvalidField.into(),
            SenderAttachError::AddressIsNoneWhenDynamicIsTrue => AmqpError::InvalidField.into(),

            SenderAttachError::IncomingSourceIsNone 
            | SenderAttachError::IncomingTargetIsNone 
            | SenderAttachError::DesireTxnCapabilitiesNotSupported => return Err(value),
        };

        Ok(Self::new(condition, format!("{:?}", value), None))
    }
}

/// Errors with sending attach
pub(crate) enum SendAttachErrorKind {
    /// Illegal link state
    IllegalState, 

    /// Illegal session state
    IllegalSessionState,
}

impl From<SendAttachErrorKind> for SenderAttachError {
    fn from(value: SendAttachErrorKind) -> Self {
        match value {
            SendAttachErrorKind::IllegalState => Self::IllegalState,
            SendAttachErrorKind::IllegalSessionState => Self::IllegalSessionState,
        }
    }
}

impl From<SendAttachErrorKind> for ReceiverAttachError {
    fn from(value: SendAttachErrorKind) -> Self {
        match value {
            SendAttachErrorKind::IllegalState => Self::IllegalState,
            SendAttachErrorKind::IllegalSessionState => Self::IllegalSessionState,
        }
    }
}

/// Errors associated with sending/handling Flow 
#[derive(Debug, thiserror::Error)]
pub enum SenderFlowError {
    /// ILlegal link state
    #[error("Illegal local state")]
    IllegalState,

    /// Session has dropped
    #[error("Session has dropped")]
    IllegalSessionState,
}

/// Errors associated with sending Transfer
#[derive(Debug, thiserror::Error)]
pub enum SenderTransferError {
    /// ILlegal link state
    #[error("Illegal local state")]
    IllegalState,

    /// Session has dropped
    #[error("Session has dropped")]
    IllegalSessionState,

    /// Remote peer detached 
    #[error("Remote detached")]
    RemoteDetached,

    /// Remote peer detached with error
    #[error("Remote detached with an error: {}", .0)]
    RemoteDetachedWithError(definitions::Error),

    /// Remote peer closed 
    #[error("Remote closed")]
    RemoteClosed,

    /// Remote peer closed the link with an error
    #[error("Remote peer closed the link with an error: {}", .0)]
    RemoteClosedWithError(definitions::Error),

    /// The link is expected to be detached immediately but didn't receive
    /// an incoming Detach frame
    #[error("Expecting an immediate detach")]
    ExpectImmediateDetach,
}

impl From<DetachError> for SenderTransferError {
    fn from(value: DetachError) -> Self {
        match value {
            DetachError::IllegalState => Self::IllegalState,
            DetachError::IllegalSessionState => Self::IllegalSessionState,
            DetachError::RemoteDetachedWithError(error) => Self::RemoteDetachedWithError(error),
            DetachError::ClosedByRemote => Self::RemoteClosed,
            DetachError::DetachedByRemote => Self::RemoteDetached,
            DetachError::RemoteClosedWithError(error) => Self::RemoteClosedWithError(error),
            DetachError::NonDetachFrameReceived => Self::ExpectImmediateDetach,
        }
    }
}

/// Errors associated with sending/handling Disposition
#[derive(Debug, thiserror::Error)]
pub enum SenderDispositionError {
    /// ILlegal link state
    #[error("Illegal local state")]
    IllegalState,

    /// Session has dropped
    #[error("Session has dropped")]
    IllegalSessionState,
}