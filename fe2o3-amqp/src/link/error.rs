use fe2o3_amqp_types::definitions::{self, AmqpError, ErrorCondition, SessionError};
use serde_amqp::primitives::Symbol;

use crate::{connection::ConnectionStopReason, session::error::AllocLinkError};

#[cfg(docsrs)]
use fe2o3_amqp_types::transaction::Coordinator;

use super::{delivery::DeliveryInfo, receiver::DetachedReceiver, sender::DetachedSender};

/// Why the link's session (or its connection) stopped before the link was
/// detached or closed. From a link's perspective, a parent stopping earlier
/// is always an error for the link's operations; the variants here describe
/// the parent's state so the caller can decide how to recover.
///
/// The unprefixed variants describe the local side's action; the `Remote*`
/// variants describe a remote-initiated end. `ConnectionStopped(..)` embeds
/// the connection's own stop reason (see [`ConnectionStopReason`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStopReason {
    /// The session ended cleanly (locally)
    Ended,
    /// We ended the session with this error
    EndedWithError(definitions::Error),
    /// The remote peer ended the session cleanly
    RemoteEnded,
    /// The remote peer ended the session with this error
    RemoteEndedWithError(definitions::Error),
    /// The connection stopped; the embedded reason tells whether the close
    /// was local or remote and whether it carried an error
    ConnectionStopped(ConnectionStopReason),
}

/// Error associated with detaching
#[derive(Debug, thiserror::Error)]
pub enum DetachError {
    /// ILlegal link state
    #[error("Illegal local state")]
    IllegalState,

    /// The session (or its connection) stopped before the link was detached
    #[error("The session stopped before the link was detached: {:?}", .0)]
    SessionStopped(SessionStopReason),

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
    /// The session (or its connection) stopped before the attach completed
    #[error("The session stopped before the link was attached: {:?}", .0)]
    SessionStopped(SessionStopReason),

    /// The session is not in the `Mapped` state (e.g. not begun, or ending)
    #[error("The session is not in a state that permits link attachment")]
    SessionNotMapped,

    /// Link name duplicated
    #[error("Link name is not unique.")]
    DuplicatedLinkName,

    /// Illegal link state
    #[error("Illegal link state")]
    IllegalState,

    /// The local terminus is expecting an Attach from the remote peer
    #[error("Expecting an Attach frame but received a non-Attach frame")]
    NonAttachFrameReceived,

    /// The link is expected to be detached immediately but didn't receive
    /// an incoming Detach frame
    #[error("Expecting the remote peer to immediately detach")]
    ExpectImmediateDetach,

    /// Incoming Attach frame's Target field is None
    #[error("Target field is None")]
    IncomingTargetIsNone,

    /// The remote Attach contains a [`Coordinator`] in the Target
    #[error("Control link is not implemented without enabling the `transaction` feature")]
    CoordinatorIsNotImplemented,

    /// The sender requested a definite settlement mode (`settled` or `unsettled`)
    /// that conflicts with the receiver's declared *desired* settlement mode in
    /// the attach response.
    ///
    /// The `snd-settle-mode` field in the attach response from the receiver only
    /// expresses the receiver's desired settlement mode for the sender. When the
    /// sender initiates the attach, the sender's own choice is the settlement
    /// mode in use, and the receiver SHOULD respect it. A response of `mixed` is
    /// tolerated regardless of the sender's choice, so this error is only
    /// produced when neither side declares `mixed` and the two definite values
    /// differ, e.g. the sender requests `settled` while the receiver responds
    /// `unsettled` (or vice versa). Such a conflict signals a receiver that
    /// expects settlement behavior the sender will not provide, so the attach
    /// is rejected with this error instead of risking broken settlement at
    /// delivery time.
    #[error(
        "The requested snd-settle-mode conflicts with the remote peer's desired settlement mode"
    )]
    SndSettleModeNotSupported,

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

cfg_transaction! {
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

    impl From<tokio::sync::TryLockError> for SenderTryConsumeError {
        fn from(_: tokio::sync::TryLockError) -> Self {
            Self::TryLockError
        }
    }
}

/// The desired filter(s) on the receiver is not supported by the remote peer
#[derive(Debug)]
pub struct DesiredFilterNotSupported {
    /// The desired filter(s)
    pub not_supported: Vec<Symbol>,
}

impl std::fmt::Display for DesiredFilterNotSupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Desired filter(s) {:?} are not supported.",
            self.not_supported
        )
    }
}

impl std::error::Error for DesiredFilterNotSupported {}

/// Errors associated with attaching a link as receiver
#[derive(Debug, thiserror::Error)]
pub enum ReceiverAttachError {
    /// The session (or its connection) stopped before the attach completed
    #[error("The session stopped before the link was attached: {:?}", .0)]
    SessionStopped(SessionStopReason),

    /// The session is not in the `Mapped` state (e.g. not begun, or ending)
    #[error("The session is not in a state that permits link attachment")]
    SessionNotMapped,

    /// Link name is already in use
    #[error("Link name is not unique.")]
    DuplicatedLinkName,

    /// Illegal link state
    #[error("Illegal link state")]
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

    /// The desired filter(s) on the receiver is not supported by the remote peer
    #[error("{:?}", .0)]
    DesiredFilterNotSupported(#[from] DesiredFilterNotSupported),
}

impl From<AllocLinkError> for ReceiverAttachError {
    fn from(value: AllocLinkError) -> Self {
        match value {
            AllocLinkError::SessionNotMapped => Self::SessionNotMapped,
            AllocLinkError::SessionStopped(reason) => Self::SessionStopped(reason),
            AllocLinkError::DuplicatedLinkName => Self::DuplicatedLinkName,
        }
    }
}

impl<'a> TryFrom<&'a ReceiverAttachError> for definitions::Error {
    type Error = &'a ReceiverAttachError;

    fn try_from(value: &'a ReceiverAttachError) -> Result<Self, Self::Error> {
        let condition: ErrorCondition = match value {
            ReceiverAttachError::SessionStopped(_) => AmqpError::IllegalState.into(),
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
            AllocLinkError::SessionNotMapped => Self::SessionNotMapped,
            AllocLinkError::SessionStopped(reason) => Self::SessionStopped(reason),
            AllocLinkError::DuplicatedLinkName => Self::DuplicatedLinkName,
        }
    }
}

impl TryFrom<DetachError> for SenderAttachError {
    type Error = DetachError;

    fn try_from(value: DetachError) -> Result<Self, Self::Error> {
        match value {
            DetachError::IllegalState => Ok(Self::IllegalState),
            DetachError::SessionStopped(reason) => Ok(Self::SessionStopped(reason)),
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
            DetachError::SessionStopped(reason) => Ok(Self::SessionStopped(reason)),
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
            SenderAttachError::SessionStopped(_) => AmqpError::IllegalState.into(),
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

            #[cfg(feature = "transaction")]
            SenderAttachError::DesireTxnCapabilitiesNotSupported => return Err(value),

            _ => return Err(value),
        };

        Ok(Self::new(condition, format!("{:?}", value), None))
    }
}

/// Errors associated with link state
#[derive(Debug, thiserror::Error)]
pub enum LinkStateError {
    /// ILlegal link state
    #[error("Illegal local state")]
    IllegalState,

    /// The session (or its connection) stopped before the link was detached or closed
    #[error("The session stopped before the link was detached or closed: {:?}", .0)]
    SessionStopped(SessionStopReason),

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
            DetachError::SessionStopped(reason) => Self::SessionStopped(reason),
            DetachError::RemoteDetachedWithError(error) => Self::RemoteDetachedWithError(error),
            DetachError::ClosedByRemote => Self::RemoteClosed,
            DetachError::DetachedByRemote => Self::RemoteDetached,
            DetachError::RemoteClosedWithError(error) => Self::RemoteClosedWithError(error),
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
    MessageDecode(#[from] MessageDecodeError),

    /// If the negotiated link value is first, then it is illegal to set this
    /// field to second.
    #[error("Negotiated value is first. Setting mode to second is illegal")]
    IllegalRcvSettleModeInTransfer,

    /// Field is inconsisten in multi-frame delivery
    #[error("Field is inconsisten in multi-frame delivery")]
    InconsistentFieldInMultiFrameDelivery,
}

/// Error decoding message
#[derive(Debug)]
pub struct MessageDecodeError {
    /// Delivery info
    pub info: DeliveryInfo,

    /// Source error
    pub source: serde_amqp::Error,
}

impl std::fmt::Display for MessageDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.info, self.source)
    }
}

impl std::error::Error for MessageDecodeError {}

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
    MessageDecode(#[from] MessageDecodeError),

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
            ReceiverTransferError::MessageDecode(err) => RecvError::MessageDecode(err),
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

    /// The session (or its connection) stopped before the link was detached or closed
    #[error("The session stopped before the link was detached or closed: {:?}", .0)]
    SessionStopped(SessionStopReason),
}

pub(crate) type SendAttachErrorKind = IllegalLinkStateError;

impl From<IllegalLinkStateError> for LinkStateError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => LinkStateError::IllegalState,
            IllegalLinkStateError::SessionStopped(reason) => LinkStateError::SessionStopped(reason),
        }
    }
}

impl From<IllegalLinkStateError> for ReceiverAttachError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => ReceiverAttachError::IllegalState,
            IllegalLinkStateError::SessionStopped(reason) => {
                ReceiverAttachError::SessionStopped(reason)
            }
        }
    }
}

impl From<IllegalLinkStateError> for SenderAttachError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => SenderAttachError::IllegalState,
            IllegalLinkStateError::SessionStopped(reason) => {
                SenderAttachError::SessionStopped(reason)
            }
        }
    }
}

impl From<IllegalLinkStateError> for SendError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => LinkStateError::IllegalState.into(),
            IllegalLinkStateError::SessionStopped(reason) => {
                LinkStateError::SessionStopped(reason).into()
            }
        }
    }
}

impl From<IllegalLinkStateError> for DetachError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => Self::IllegalState,
            IllegalLinkStateError::SessionStopped(reason) => Self::SessionStopped(reason),
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

impl std::fmt::Display for SenderResumeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SenderResumeError: {}", self.kind)
    }
}

impl std::error::Error for SenderResumeError {}

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

impl std::fmt::Display for ReceiverResumeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReceiverResumeError: {}", self.kind)
    }
}

impl std::error::Error for ReceiverResumeError {}

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

/// Error with `Sender::detach_then_resume_on_session`
#[derive(Debug, thiserror::Error)]
pub enum DetachThenResumeSenderError {
    /// Error with detaching the sender
    #[error(transparent)]
    Detach(#[from] DetachError),

    /// Error with resuming the sender
    #[error(transparent)]
    Resume(#[from] SenderResumeErrorKind),
}

/// Error with `Receiver::detach_then_resume_on_session`
#[derive(Debug, thiserror::Error)]
pub enum DetachThenResumeReceiverError {
    /// Error with detaching the receiver
    #[error(transparent)]
    Detach(#[from] DetachError),

    /// Error with resuming the receiver
    #[error(transparent)]
    Resume(#[from] ReceiverResumeErrorKind),
}
