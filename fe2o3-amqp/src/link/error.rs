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

/// The recovery action for a session that stopped.
fn session_stop_recovery(reason: &SessionStopReason) -> ErrorRecovery {
    match reason {
        SessionStopReason::ConnectionStopped(_) => ErrorRecovery::ReconnectConnection,
        _ => ErrorRecovery::ReconnectSession,
    }
}

/// What a caller can do with a link after an operation failed.
///
/// Returned by the `recovery()` method on [`LinkStateError`], [`DetachError`],
/// [`SendError`] and [`RecvError`]. The action is derived from the error alone;
/// defensive errors that can cover several link states are classified
/// conservatively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorRecovery {
    /// The link is still attached and usable; the error is scoped to the
    /// operation or delivery.
    UseLink,

    /// The link is detached and can be resumed: `detach()` (idempotent) then
    /// `resume()`, `resume_on_session()`, or `detach_then_resume_on_session()`.
    ReattachLink,

    /// The session stopped; obtain a new session and resume the link on it
    /// with `resume_on_session()` or `detach_then_resume_on_session()`.
    ReconnectSession,

    /// The connection stopped too; obtain a new connection and session and
    /// resume the link on the new session.
    ReconnectConnection,

    /// The link was closed or destroyed, or the error left its state
    /// indeterminate (the defensive `IllegalState`/`ExpectImmediateDetach`
    /// variants): create a new link.
    NewLink,
}

impl ErrorRecovery {
    /// Whether the link can still be used as-is.
    pub fn link_is_usable(self) -> bool {
        matches!(self, Self::UseLink)
    }

    /// Whether the link must be resumed, possibly on a new session or
    /// connection, before it can be used again.
    pub fn requires_reattach(self) -> bool {
        matches!(
            self,
            Self::ReattachLink | Self::ReconnectSession | Self::ReconnectConnection
        )
    }

    /// Whether the link is gone and a new link must be created.
    pub fn requires_new_link(self) -> bool {
        matches!(self, Self::NewLink)
    }
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

    /// The remote peer closed the link with a closing detach (`closed=true`).
    ///
    /// When the closing detach answers a non-closing detach from this side, the
    /// link is reattached and then closed, completing the handshake (AMQP 1.0
    /// §2.6.6). The link is left `Closed`.
    #[error("Link closed by remote")]
    ClosedByRemote,

    /// Remote peer closed the link with an error
    #[error("Remote peer closed the link with an error: {}", .0)]
    RemoteClosedWithError(definitions::Error),
}

/// How the peer detached the link.
///
/// This is an outcome, not a local failure: the peer either suspended the
/// link with a non-closing detach or destroyed it with a closing detach. The
/// `error` field the peer attached to its detach, if any, is carried in
/// `remote_error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetachStatus {
    /// The peer suspended the link with a non-closing detach.
    Detached {
        /// The error the peer attached to its detach, if any.
        remote_error: Option<definitions::Error>,
    },

    /// The peer destroyed the link with a closing detach.
    Closed {
        /// The error the peer attached to its closing detach, if any.
        remote_error: Option<definitions::Error>,
    },
}

impl DetachStatus {
    /// Whether the peer destroyed the link (`true`) or suspended it (`false`).
    pub fn is_closed(&self) -> bool {
        matches!(self, DetachStatus::Closed { .. })
    }

    /// The error the peer attached to its detach, if any.
    pub fn remote_error(&self) -> Option<&definitions::Error> {
        match self {
            DetachStatus::Detached { remote_error } | DetachStatus::Closed { remote_error } => {
                remote_error.as_ref()
            }
        }
    }
}

/// Error from recording a peer-initiated detach with
/// [`LinkDetach::apply_remote_detach_outcome`].
///
/// The outcome itself is reported as [`DetachStatus`]; this error means the
/// outcome was *not* recorded: the link is `Unattached`, already `Detached`,
/// or already `Closed`, and nothing changed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ApplyRemoteDetachError {
    /// The outcome was not recorded and nothing changed.
    #[error("Illegal link state")]
    IllegalState,
}

impl From<ApplyRemoteDetachError> for DetachError {
    fn from(_: ApplyRemoteDetachError) -> Self {
        DetachError::IllegalState
    }
}

impl From<ApplyRemoteDetachError> for LinkStateError {
    fn from(_: ApplyRemoteDetachError) -> Self {
        LinkStateError::IllegalState
    }
}

/// Failure of a sender transfer.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TransferError {
    /// A local link-state failure
    #[error(transparent)]
    LinkState(#[from] LinkStateError),

    /// The peer detached the link before the transfer completed
    #[error("The peer detached the link")]
    LinkEnded(DetachStatus),

    /// Expecting the peer to detach immediately but received another frame
    #[error("Expecting the peer to immediately detach")]
    ExpectImmediateDetach,
}

impl From<TransferError> for SendError {
    fn from(value: TransferError) -> Self {
        match value {
            TransferError::LinkState(error) => SendError::LinkStateError(error),
            TransferError::LinkEnded(status) => SendError::LinkEnded(status),
            TransferError::ExpectImmediateDetach => {
                SendError::LinkStateError(LinkStateError::ExpectImmediateDetach)
            }
        }
    }
}

/// Failure delivered through a delivery's settlement channel.
#[derive(Debug, Clone)]
pub(crate) enum DeliveryFailure {
    /// A local link-state failure
    LinkState(LinkStateError),
    /// The peer detached the link while the delivery was pending
    LinkEnded(DetachStatus),
}

impl From<DeliveryFailure> for SendError {
    fn from(value: DeliveryFailure) -> Self {
        match value {
            DeliveryFailure::LinkState(error) => SendError::LinkStateError(error),
            DeliveryFailure::LinkEnded(status) => SendError::LinkEnded(status),
        }
    }
}

impl From<DeliveryFailure> for RecvError {
    fn from(value: DeliveryFailure) -> Self {
        match value {
            DeliveryFailure::LinkState(error) => RecvError::LinkStateError(error),
            DeliveryFailure::LinkEnded(status) => RecvError::LinkEnded(status),
        }
    }
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

/// The encoded message is larger than the maximum message size negotiated on
/// the link.
///
/// The maximum message size is the minimum of the local and remote
/// `max-message-size` advertised at link attach (see `get_max_message_size`);
/// a value of zero means no limit is imposed. The error is produced on both
/// sides of the link:
///
/// - On the **sender** side, [`Sender::send`](crate::Sender::send) rejects
///   the message locally before any transfer frame is sent, mirroring other
///   AMQP client implementations (e.g. go-amqp surfaces the
///   `amqp:link:message-size-exceeded` error condition in this case).
/// - On the **receiver** side, [`Receiver::recv`](crate::Receiver::recv)
///   returns this error when the peer sends a delivery larger than the
///   advertised `max_message_size`. Receiving an oversized message is a link
///   error (AMQP 1.0 §2.7.3, §2.8.18), so the link is detached with
///   `amqp:link:message-size-exceeded` and must be resumed to be used again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageSizeExceeded {
    /// Size of the encoded message in bytes
    pub size: u64,
    /// The maximum message size of the link in bytes
    pub max_size: u64,
}

impl std::fmt::Display for MessageSizeExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "message size exceeds max of {}", self.max_size)
    }
}

impl std::error::Error for MessageSizeExceeded {}

/// Error associated with sending a message
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    /// Errors found in link state
    #[error("Local error: {:?}", .0)]
    LinkStateError(#[from] LinkStateError),

    /// The peer detached the link before the delivery was settled
    #[error("The peer detached the link: {:?}", .0)]
    LinkEnded(DetachStatus),

    /// A non-terminal delivery state is received while expecting
    /// an outcome
    #[error("A non-terminal delivery state is received when an outcome is expected")]
    NonTerminalDeliveryState,

    /// Transactional state found on non-transactional delivery
    #[error("Transactional state found on non-transactional delivery")]
    IllegalDeliveryState,

    /// The encoded message is larger than the maximum message size
    /// negotiated on the link
    #[error(transparent)]
    MessageSizeExceeded(MessageSizeExceeded),

    /// Error serializing message
    #[error("Error encoding message")]
    MessageEncodeError,
}

impl From<serde_amqp::Error> for SendError {
    fn from(_: serde_amqp::Error) -> Self {
        Self::MessageEncodeError
    }
}

impl From<MessageSizeExceeded> for SendError {
    fn from(error: MessageSizeExceeded) -> Self {
        Self::MessageSizeExceeded(error)
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
            DetachError::ClosedByRemote => Err(value),
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
            DetachError::ClosedByRemote => Err(value),
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
#[derive(Debug, Clone, thiserror::Error)]
pub enum LinkStateError {
    /// ILlegal link state
    #[error("Illegal local state")]
    IllegalState,

    /// The session (or its connection) stopped before the link was detached or closed
    #[error("The session stopped before the link was detached or closed: {:?}", .0)]
    SessionStopped(SessionStopReason),

    /// The link is expected to be detached immediately but didn't receive
    /// an incoming Detach frame
    #[error("Expecting an immediate detach")]
    ExpectImmediateDetach,
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

    /// The peer detached the link before a delivery could be received
    #[error("The peer detached the link: {:?}", .0)]
    LinkEnded(DetachStatus),

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

    /// A delivery larger than the negotiated `max_message_size` of the link
    /// was received. The link is detached with the
    /// `amqp:link:message-size-exceeded` error condition (AMQP 1.0 §2.6.5) and
    /// can only be restored by resuming it.
    #[error(transparent)]
    MessageSizeExceeded(MessageSizeExceeded),

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

impl RecvError {
    /// Classifies what the caller can do with the link after this error.
    pub fn recovery(&self) -> ErrorRecovery {
        match self {
            Self::LinkStateError(error) => error.recovery(),
            Self::TransferLimitExceeded
            | Self::MessageDecode(_)
            | Self::IllegalRcvSettleModeInTransfer => ErrorRecovery::UseLink,
            Self::DeliveryIdIsNone
            | Self::DeliveryTagIsNone
            | Self::MessageSizeExceeded(_)
            | Self::InconsistentFieldInMultiFrameDelivery
            | Self::TransactionalAcquisitionIsNotImeplemented => ErrorRecovery::NewLink,
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

#[cfg(test)]
mod tests {
    use fe2o3_amqp_types::definitions::DeliveryTag;

    use super::*;
    use crate::util::Sealed;

    fn test_error() -> definitions::Error {
        definitions::Error::new(definitions::AmqpError::InternalError, None, None)
    }

    fn message_decode_error() -> MessageDecodeError {
        let info = DeliveryInfo {
            delivery_id: 0,
            delivery_tag: DeliveryTag::from(vec![0x01]),
            rcv_settle_mode: None,
            _sealed: Sealed {},
        };
        let source = serde_amqp::from_slice::<String>(&[]).unwrap_err();
        MessageDecodeError { info, source }
    }

    #[test]
    fn link_state_error_recovery() {
        assert_eq!(
            LinkStateError::RemoteDetached.recovery(),
            ErrorRecovery::ReattachLink
        );
        assert_eq!(
            LinkStateError::RemoteDetachedWithError(test_error()).recovery(),
            ErrorRecovery::ReattachLink
        );
        assert_eq!(
            LinkStateError::RemoteClosed.recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            LinkStateError::RemoteClosedWithError(test_error()).recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            LinkStateError::IllegalState.recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            LinkStateError::ExpectImmediateDetach.recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            LinkStateError::SessionStopped(SessionStopReason::Ended).recovery(),
            ErrorRecovery::ReconnectSession
        );
        assert_eq!(
            LinkStateError::SessionStopped(SessionStopReason::ConnectionStopped(
                ConnectionStopReason::Closed
            ))
            .recovery(),
            ErrorRecovery::ReconnectConnection
        );
    }

    #[test]
    #[allow(deprecated)]
    fn detach_error_recovery() {
        assert_eq!(
            DetachError::SessionStopped(SessionStopReason::RemoteEnded).recovery(),
            ErrorRecovery::ReconnectSession
        );
        assert_eq!(
            DetachError::SessionStopped(SessionStopReason::ConnectionStopped(
                ConnectionStopReason::RemoteClosed
            ))
            .recovery(),
            ErrorRecovery::ReconnectConnection
        );
        assert_eq!(
            DetachError::RemoteDetachedWithError(test_error()).recovery(),
            ErrorRecovery::ReattachLink
        );
        assert_eq!(
            DetachError::ClosedByRemote.recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            DetachError::RemoteClosedWithError(test_error()).recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(DetachError::IllegalState.recovery(), ErrorRecovery::NewLink);
        assert_eq!(
            DetachError::DetachedByRemote.recovery(),
            ErrorRecovery::ReattachLink
        );
    }

    #[test]
    fn send_error_recovery() {
        assert_eq!(
            SendError::LinkStateError(LinkStateError::RemoteDetached).recovery(),
            ErrorRecovery::ReattachLink
        );
        assert_eq!(
            SendError::Detached(DetachError::ClosedByRemote).recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            SendError::NonTerminalDeliveryState.recovery(),
            ErrorRecovery::UseLink
        );
        assert_eq!(
            SendError::IllegalDeliveryState.recovery(),
            ErrorRecovery::UseLink
        );
        assert_eq!(
            SendError::MessageSizeExceeded(MessageSizeExceeded {
                size: 10,
                max_size: 1
            })
            .recovery(),
            ErrorRecovery::UseLink
        );
        assert_eq!(
            SendError::MessageEncodeError.recovery(),
            ErrorRecovery::UseLink
        );
    }

    #[test]
    fn recv_error_recovery() {
        assert_eq!(
            RecvError::LinkStateError(LinkStateError::SessionStopped(
                SessionStopReason::ConnectionStopped(ConnectionStopReason::Closed)
            ))
            .recovery(),
            ErrorRecovery::ReconnectConnection
        );
        assert_eq!(
            RecvError::TransferLimitExceeded.recovery(),
            ErrorRecovery::UseLink
        );
        assert_eq!(
            RecvError::MessageDecode(message_decode_error()).recovery(),
            ErrorRecovery::UseLink
        );
        assert_eq!(
            RecvError::IllegalRcvSettleModeInTransfer.recovery(),
            ErrorRecovery::UseLink
        );
        assert_eq!(
            RecvError::DeliveryIdIsNone.recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            RecvError::DeliveryTagIsNone.recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            RecvError::MessageSizeExceeded(MessageSizeExceeded {
                size: 10,
                max_size: 1
            })
            .recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            RecvError::InconsistentFieldInMultiFrameDelivery.recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            RecvError::TransactionalAcquisitionIsNotImeplemented.recovery(),
            ErrorRecovery::NewLink
        );
    }

    #[test]
    fn recovery_predicates() {
        assert!(ErrorRecovery::UseLink.link_is_usable());
        assert!(!ErrorRecovery::UseLink.requires_reattach());
        assert!(!ErrorRecovery::UseLink.requires_new_link());

        assert!(ErrorRecovery::ReattachLink.requires_reattach());
        assert!(ErrorRecovery::ReconnectSession.requires_reattach());
        assert!(ErrorRecovery::ReconnectConnection.requires_reattach());

        assert!(ErrorRecovery::NewLink.requires_new_link());
        assert!(!ErrorRecovery::NewLink.link_is_usable());
    }
}
