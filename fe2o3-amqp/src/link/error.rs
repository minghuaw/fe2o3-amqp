use fe2o3_amqp_types::definitions::{self, AmqpError, ErrorCondition, SessionError};
use tokio::sync::TryLockError;

use crate::session::AllocLinkError;

#[cfg(docsrs)]
use fe2o3_amqp_types::transaction::Coordinator;

use super::{receiver::DetachedReceiver, sender::DetachedSender};

/// Error associated with detaching
#[derive(Debug, thiserror::Error)]
pub enum DetachError {
    /// ILlegal link state
    #[error("Illegal local state")]
    IllegalState,

    /// Session has dropped
    #[error("Session has dropped")]
    IllegalSessionState,

    // /// Expecting a detach but found other frame
    // #[error("Expecting a Detach")]
    // NonDetachFrameReceived,
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
    RemoteClosedWithError(definitions::Error),
}

/// Errors associated with attaching a link as sender
#[derive(Debug, thiserror::Error)]
pub enum SenderAttachError {
    // Illegal session state
    /// Session stopped
    #[error("Illegal session state. Session might have stopped.")]
    IllegalSessionState,

    /// Link name duplicated
    #[error("Link name is not unique.")]
    DuplicatedLinkName,

    /// Illegal link state
    #[error("Illegal session state")]
    IllegalState,

    /// The local terminus is expecting an Attach from the remote peer
    #[error("Expecting an Attach frame but received a non-Attach frame")]
    NonAttachFrameReceived,

    /// The link is expected to be detached immediately but didn't receive
    /// an incoming Detach frame
    #[error("Expecting the remote peer to immediately detach")]
    ExpectImmediateDetach,

    // Errors that should reject Attach
    /// Incoming Attach frame's Source field is None
    #[deprecated = "Since 0.7.1, `source` from a receiver link is not checked at the sender anymore"]
    #[error("Source field is None")]
    IncomingSourceIsNone,

    /// Incoming Attach frame's Target field is None
    #[error("Target field is None")]
    IncomingTargetIsNone,

    /// The remote Attach contains a [`Coordinator`] in the Target
    #[error("Control link is not implemented without enabling the `transaction` feature")]
    CoordinatorIsNotImplemented,

    /// When set at the sender this indicates the actual settlement mode in use.
    ///
    /// The sender SHOULD respect the receiver’s desired settlement mode ***if
    /// the receiver initiates*** the attach exchange and the sender supports the desired mode
    #[error("When set at the sender this indicates the actual settlement mode in use")]
    SndSettleModeNotSupported,

    /// "When set at the receiver this indicates the actual settlement mode in use"
    ///
    /// The receiver SHOULD respect the sender’s desired settlement mode ***if
    /// the sender initiates*** the attach exchange and the receiver supports the desired mode
    #[error("The desried ReceiverSettleMode is not supported by the remote peer")]
    RcvSettleModeNotSupported,

    /// When set to true by the receiving link endpoint this field indicates creation of a
    /// dynamically created node. In this case the address field will contain the address of the
    /// created node.
    #[error("The address field contins the address of the created node when dynamic is set by the receiving endpoint")]
    TargetAddressIsNoneWhenDynamicIsTrue,

    /// When set to true by the receiving link endpoint, this field constitutes a request for the sending
    /// peer to dynamically create a node at the source. In this case the address field MUST NOT be set
    #[error("Source address must not be set when dynamic is set by the receiving endpoint")]
    SourceAddressIsSomeWhenDynamicIsTrue,

    /// If the dynamic field is not set to true this field MUST be left unset.
    #[error("If the dynamic field is not set to true this field MUST be left unset")]
    DynamicNodePropertiesIsSomeWhenDynamicIsFalse,

    /// Desired TransactionCapabilities is not supported
    #[cfg(feature = "transaction")]
    #[error("Desired transaction capability is not supported")]
    DesireTxnCapabilitiesNotSupported,

    /// Remote peer closed the link with an error
    #[error("Remote peer closed with error {:?}", .0)]
    RemoteClosedWithError(definitions::Error),
}

/// Error associated with sending a message
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    /// Errors found in link state
    #[error("Local error: {:?}", .0)]
    LinkStateError(#[from] LinkStateError),

    /// The remote peer detached with error
    #[error("Link is detached {:?}", .0)]
    Detached(DetachError),

    /// A non-terminal delivery state is received while expecting
    /// an outcome
    #[error("A non-terminal delivery state is received when an outcome is expected")]
    NonTerminalDeliveryState,

    /// Transactional state found on non-transactional delivery
    #[error("Transactional state found on non-transactional delivery")]
    IllegalDeliveryState,

    /// Error serializing message
    #[error("Error encoding message")]
    MessageEncodeError,
}

impl From<serde_amqp::Error> for SendError {
    fn from(_: serde_amqp::Error) -> Self {
        Self::MessageEncodeError
    }
}

impl From<DetachError> for SendError {
    fn from(error: DetachError) -> Self {
        Self::Detached(error)
    }
}

/// Error with the sender trying consume link credit
///
/// This is only used in
#[derive(Debug, thiserror::Error)]
pub(crate) enum SenderTryConsumeError {
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
#[derive(Debug, thiserror::Error)]
pub enum ReceiverAttachError {
    // Errors that should end the session
    /// The associated session has dropped
    #[error("Illegal session state. Session might have stopped.")]
    IllegalSessionState,

    /// Link name is already in use
    #[error("Link name is not unique.")]
    DuplicatedLinkName,

    /// Illegal link state
    #[error("Illegal session state")]
    IllegalState,

    /// The local terminus is expecting an Attach from the remote peer
    #[error("Expecting an Attach frame but received a non-Attach frame")]
    NonAttachFrameReceived,

    /// The link is expected to be detached immediately but didn't receive
    /// an incoming Detach frame
    #[error("Expecting the remote peer to immediately detach")]
    ExpectImmediateDetach,

    // Errors that should reject Attach
    /// Incoming Attach frame's Source field is None
    #[error("Source field is None")]
    IncomingSourceIsNone,

    /// Incoming Attach frame's Target field is None
    #[deprecated = "Since 0.7.1 `target` from a sender link is not checked at the receiver anymore"]
    #[error("Target field is None")]
    IncomingTargetIsNone,

    /// The remote Attach contains a [`Coordinator`] in the Target
    #[error("Control link is not implemented without enabling the `transaction` feature")]
    CoordinatorIsNotImplemented,

    /// This MUST NOT be null if role is sender
    #[error("Initial delivery field must be set if the role is sender")]
    InitialDeliveryCountIsNone,

    // /// When set at the sender this indicates the actual settlement mode in use.
    // ///
    // /// The sender SHOULD respect the receiver’s desired settlement mode ***if
    // /// the receiver initiates*** the attach exchange and the sender supports the desired mode
    // #[error("When set at the sender this indicates the actual settlement mode in use")]
    // SndSettleModeNotSupported,
    /// "When set at the receiver this indicates the actual settlement mode in use"
    ///
    /// The receiver SHOULD respect the sender’s desired settlement mode ***if
    /// the sender initiates*** the attach exchange and the receiver supports the desired mode
    #[error("The desried ReceiverSettleMode is not supported by the remote peer")]
    RcvSettleModeNotSupported,

    /// When dynamic is set to true by the sending link endpoint, this field constitutes a request
    /// for the receiving peer to dynamically create a node at the target. In this case the address
    /// field MUST NOT be set.
    #[error("Target address MUST not be set when dynamic is set to by a sending link endpoint")]
    TargetAddressIsSomeWhenDynamicIsTrue,

    /// When set to true by the sending link endpoint this field indicates creation of a dynamically created
    /// node. In this case the address field will contain the address of the created node
    #[error("When set to true by the sending link endpoint this field indicates creation of a dynamically created node")]
    SourceAddressIsNoneWhenDynamicIsTrue,

    /// If the dynamic field is not set to true this field MUST be left unset.
    #[error("If the dynamic field is not set to true this field MUST be left unset")]
    DynamicNodePropertiesIsSomeWhenDynamicIsFalse,

    /// Remote peer closed the link with an error
    #[error("Remote peer closed with error {:?}", .0)]
    RemoteClosedWithError(definitions::Error),
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
            ReceiverAttachError::CoordinatorIsNotImplemented => AmqpError::NotImplemented.into(),
            ReceiverAttachError::InitialDeliveryCountIsNone => AmqpError::InvalidField.into(),
            ReceiverAttachError::TargetAddressIsSomeWhenDynamicIsTrue => {
                AmqpError::InvalidField.into()
            }
            ReceiverAttachError::SourceAddressIsNoneWhenDynamicIsTrue => {
                AmqpError::InvalidField.into()
            }
            ReceiverAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse => {
                AmqpError::InvalidField.into()
            }
            _ => return Err(value),
        };

        Ok(Self::new(condition, format!("{:?}", value), None))
    }
}

impl From<AllocLinkError> for SenderAttachError {
    fn from(value: AllocLinkError) -> Self {
        match value {
            AllocLinkError::IllegalSessionState => Self::IllegalSessionState,
            AllocLinkError::DuplicatedLinkName => Self::DuplicatedLinkName,
        }
    }
}

impl TryFrom<DetachError> for SenderAttachError {
    type Error = DetachError;

    fn try_from(value: DetachError) -> Result<Self, Self::Error> {
        match value {
            DetachError::IllegalState => Ok(Self::IllegalState),
            DetachError::IllegalSessionState => Ok(Self::IllegalSessionState),
            DetachError::RemoteDetachedWithError(error)
            | DetachError::RemoteClosedWithError(error) => {
                // A closing detach is used for errors during attach anyway
                Ok(Self::RemoteClosedWithError(error))
            }
            // DetachError::NonDetachFrameReceived
            DetachError::ClosedByRemote | DetachError::DetachedByRemote => Err(value),
        }
    }
}

impl TryFrom<DetachError> for ReceiverAttachError {
    type Error = DetachError;

    fn try_from(value: DetachError) -> Result<Self, Self::Error> {
        match value {
            DetachError::IllegalState => Ok(Self::IllegalState),
            DetachError::IllegalSessionState => Ok(Self::IllegalSessionState),
            DetachError::RemoteDetachedWithError(error)
            | DetachError::RemoteClosedWithError(error) => {
                // A closing detach is used for errors during attach anyway
                Ok(Self::RemoteClosedWithError(error))
            }
            // DetachError::NonDetachFrameReceived
            DetachError::ClosedByRemote | DetachError::DetachedByRemote => Err(value),
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
            SenderAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse => {
                AmqpError::InvalidField.into()
            }
            SenderAttachError::TargetAddressIsNoneWhenDynamicIsTrue => {
                AmqpError::InvalidField.into()
            }
            SenderAttachError::SourceAddressIsSomeWhenDynamicIsTrue => {
                AmqpError::InvalidField.into()
            }

            _ => return Err(value),

            #[cfg(feature = "transaction")]
            SenderAttachError::DesireTxnCapabilitiesNotSupported => return Err(value),
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

/// Errors associated with link state
#[derive(Debug, thiserror::Error)]
pub enum LinkStateError {
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

impl From<DetachError> for LinkStateError {
    fn from(value: DetachError) -> Self {
        match value {
            DetachError::IllegalState => Self::IllegalState,
            DetachError::IllegalSessionState => Self::IllegalSessionState,
            DetachError::RemoteDetachedWithError(error) => Self::RemoteDetachedWithError(error),
            DetachError::ClosedByRemote => Self::RemoteClosed,
            DetachError::DetachedByRemote => Self::RemoteDetached,
            DetachError::RemoteClosedWithError(error) => Self::RemoteClosedWithError(error),
            // DetachError::NonDetachFrameReceived => Self::ExpectImmediateDetach,
        }
    }
}

/// Errors associated with receiving a transfer
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReceiverTransferError {
    /// ILlegal link state
    #[error("Illegal local state")]
    IllegalState,

    /// The peer sent more message transfers than currently allowed on the link.
    #[error("The peer sent more message transfers than currently allowed on the link")]
    TransferLimitExceeded,

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

    /// Field is inconsisten in multi-frame delivery
    #[error("Field is inconsisten in multi-frame delivery")]
    InconsistentFieldInMultiFrameDelivery,
}

/// Errors associated with receiving
#[derive(Debug, thiserror::Error)]
pub enum RecvError {
    /// Errors found in link state
    #[error("Local error: {:?}", .0)]
    LinkStateError(LinkStateError),

    /// The peer sent more message transfers than currently allowed on the link.
    #[error("The peer sent more message transfers than currently allowed on the link")]
    TransferLimitExceeded,

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

    /// Field is inconsisten in multi-frame delivery
    #[error("Field is inconsisten in multi-frame delivery")]
    InconsistentFieldInMultiFrameDelivery,

    /// Transactional acquision is not supported yet
    #[error("Transactional acquisition is not implemented")]
    TransactionalAcquisitionIsNotImeplemented,
}

impl From<ReceiverTransferError> for RecvError {
    fn from(value: ReceiverTransferError) -> Self {
        match value {
            ReceiverTransferError::TransferLimitExceeded => RecvError::TransferLimitExceeded,
            ReceiverTransferError::DeliveryIdIsNone => RecvError::DeliveryIdIsNone,
            ReceiverTransferError::DeliveryTagIsNone => RecvError::DeliveryTagIsNone,
            ReceiverTransferError::MessageDecodeError => RecvError::MessageDecodeError,
            ReceiverTransferError::IllegalRcvSettleModeInTransfer => {
                RecvError::IllegalRcvSettleModeInTransfer
            }
            ReceiverTransferError::InconsistentFieldInMultiFrameDelivery => {
                RecvError::InconsistentFieldInMultiFrameDelivery
            }
            ReceiverTransferError::IllegalState => {
                RecvError::LinkStateError(LinkStateError::IllegalState)
            }
        }
    }
}

/// Type alias for disposition error
pub type DispositionError = IllegalLinkStateError;

/// Type alias for flow error
pub type FlowError = IllegalLinkStateError;

/// Errors associated with sending/handling Disposition
#[derive(Debug, thiserror::Error)]
pub enum IllegalLinkStateError {
    /// ILlegal link state
    #[error("Illegal local state")]
    IllegalState,

    /// Session has dropped
    #[error("Session has dropped")]
    IllegalSessionState,
}

impl From<IllegalLinkStateError> for LinkStateError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => LinkStateError::IllegalState,
            IllegalLinkStateError::IllegalSessionState => LinkStateError::IllegalSessionState,
        }
    }
}

impl From<IllegalLinkStateError> for ReceiverAttachError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => ReceiverAttachError::IllegalState,
            IllegalLinkStateError::IllegalSessionState => ReceiverAttachError::IllegalSessionState,
        }
    }
}

impl From<IllegalLinkStateError> for SenderAttachError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => SenderAttachError::IllegalState,
            IllegalLinkStateError::IllegalSessionState => SenderAttachError::IllegalSessionState,
        }
    }
}

impl From<IllegalLinkStateError> for SendError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => LinkStateError::IllegalState.into(),
            IllegalLinkStateError::IllegalSessionState => {
                LinkStateError::IllegalSessionState.into()
            }
        }
    }
}

impl From<IllegalLinkStateError> for DetachError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => Self::IllegalState,
            IllegalLinkStateError::IllegalSessionState => Self::IllegalSessionState,
        }
    }
}

impl<T> From<T> for RecvError
where
    T: Into<LinkStateError>,
{
    fn from(value: T) -> Self {
        Self::LinkStateError(value.into())
    }
}

// /// Error trying to unwrap the body section
// #[derive(Debug, thiserror::Error)]
// pub enum BodyError {
//     /// Attempting to unwrap the body as either sequence or data but body is value
//     #[error("Body is Value")]
//     IsValue,

//     /// Attempting to unwrap the body as either sequence or value but body is Data
//     #[error("Body is Data")]
//     IsData,

//     /// Attempting to unwrap the body as either value or data but body is sequence
//     #[error("Body is Sequence")]
//     IsSequence,

//     /// Attempting to unwrap the body as either sequence or value but body is DataBatch
//     ///
//     /// Added since `"0.6.0"`
//     #[error("Body is DataBatch")]
//     IsDataBatch,

//     /// Attempting to unwrap the body as either sequence or value but body is SequenceBatch
//     ///
//     /// Added since `"0.6.0"`
//     #[error("Body is SequenceBatch")]
//     IsSequenceBatch,

//     /// Attempting to unwrap the body but no body section is found
//     #[error("Body is nothing")]
//     IsEmpty,
// }

/// Errors associated with resuming a sender link endpoint
#[derive(Debug, thiserror::Error)]
pub enum SenderResumeErrorKind {
    /// Sender attach error
    #[error(transparent)]
    AttachError(#[from] SenderAttachError),

    /// Send error
    #[error(transparent)]
    SendError(#[from] SendError),

    /// Detach/suspend error
    #[error(transparent)]
    DetachError(#[from] DetachError),

    /// Resume timed out
    #[error("Resume timed out")]
    Timeout,
}

/// Sender encountered error with resumption
#[derive(Debug)]
pub struct SenderResumeError {
    /// The detached sender
    pub detached_sender: DetachedSender,

    /// The error with resumption
    pub kind: SenderResumeErrorKind,
}

/// Error kind of receiver resumption
#[derive(Debug, thiserror::Error)]
pub enum ReceiverResumeErrorKind {
    /// Error with exchanging the attach frame
    #[error(transparent)]
    AttachError(#[from] ReceiverAttachError),

    /// Error with sending flow
    #[error(transparent)]
    FlowError(#[from] IllegalLinkStateError),

    /// Detach/suspend error
    #[error(transparent)]
    DetachError(#[from] DetachError),

    /// Resume timed out
    #[error("Resume timed out")]
    Timeout,
}

/// Receiver resumption error
#[derive(Debug)]
pub struct ReceiverResumeError {
    /// The detached receiver
    pub detached_recver: DetachedReceiver,

    /// The error with resumption
    pub kind: ReceiverResumeErrorKind,
}

/// Error with link relay
#[derive(Debug, thiserror::Error)]
pub(crate) enum LinkRelayError {
    /// Link is not attached
    #[error("Link is not attached")]
    UnattachedHandle,

    /// Found a transfer frame to sender
    #[error("Found transfer frame sent to a sender")]
    TransferFrameToSender,
}

impl From<LinkRelayError> for definitions::Error {
    fn from(error: LinkRelayError) -> Self {
        match error {
            LinkRelayError::UnattachedHandle => definitions::Error {
                condition: SessionError::UnattachedHandle.into(),
                description: None,
                info: None,
            },
            LinkRelayError::TransferFrameToSender => definitions::Error {
                condition: AmqpError::NotAllowed.into(),
                description: Some(String::from("Transfer frame must not be sent to Sender")),
                info: None,
            },
        }
    }
}
