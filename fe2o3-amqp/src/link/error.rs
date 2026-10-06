use std::sync::OnceLock;

use fe2o3_amqp_types::definitions::{self, AmqpError, ErrorCondition, SessionError};
use serde_amqp::primitives::Symbol;

use crate::{connection::ConnectionOutcome, session::error::AllocLinkError};

use super::state::LinkState;

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
/// the connection's own stop reason (see [`ConnectionOutcome`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionOutcome {
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
    ConnectionStopped(ConnectionOutcome),
}

/// The recovery action for a session that stopped.
fn session_stop_recovery(reason: &SessionOutcome) -> ErrorRecovery {
    match reason {
        SessionOutcome::ConnectionStopped(_) => ErrorRecovery::ReconnectConnection,
        _ => ErrorRecovery::ReconnectSession,
    }
}

/// Warn that an operation failed after the session's relay was dropped without
/// a recorded stop reason; the failure is link-local in that case.
fn warn_unrecorded_stop_reason() {
    #[cfg(feature = "tracing")]
    tracing::warn!("session stop reason not recorded; reporting a link-local failure");
    #[cfg(feature = "log")]
    log::warn!("session stop reason not recorded; reporting a link-local failure");
}

/// The [`LinkStateError`] for an operation that failed because the session (or
/// its connection) stopped; [`LinkStateError::IllegalState`] when no stop
/// reason was recorded (defensive).
pub(crate) fn link_state_error_from_stop_reason(cell: &OnceLock<SessionOutcome>) -> LinkStateError {
    match cell.get() {
        Some(reason) => LinkStateError::SessionStopped(reason.clone()),
        None => {
            warn_unrecorded_stop_reason();
            LinkStateError::IllegalState
        }
    }
}

/// The [`DetachError`] for an operation that failed because the session (or
/// its connection) stopped; [`DetachError::IllegalState`] when no stop reason
/// was recorded (defensive).
pub(crate) fn detach_error_from_stop_reason(cell: &OnceLock<SessionOutcome>) -> DetachError {
    match cell.get() {
        Some(reason) => DetachError::SessionStopped(reason.clone()),
        None => {
            warn_unrecorded_stop_reason();
            DetachError::IllegalState
        }
    }
}

/// The [`SenderAttachError`] for an attach that failed because the session (or
/// its connection) stopped; [`SenderAttachError::IllegalState`] when no stop
/// reason was recorded (defensive).
pub(crate) fn sender_attach_error_from_stop_reason(
    cell: &OnceLock<SessionOutcome>,
) -> SenderAttachError {
    match cell.get() {
        Some(reason) => SenderAttachError::SessionStopped(reason.clone()),
        None => {
            warn_unrecorded_stop_reason();
            SenderAttachError::IllegalState
        }
    }
}

/// The [`ReceiverAttachError`] for an attach that failed because the session
/// (or its connection) stopped; [`ReceiverAttachError::IllegalState`] when no
/// stop reason was recorded (defensive).
pub(crate) fn receiver_attach_error_from_stop_reason(
    cell: &OnceLock<SessionOutcome>,
) -> ReceiverAttachError {
    match cell.get() {
        Some(reason) => ReceiverAttachError::SessionStopped(reason.clone()),
        None => {
            warn_unrecorded_stop_reason();
            ReceiverAttachError::IllegalState
        }
    }
}

/// The [`LinkStateError`] for an operation whose incoming channel closed with
/// no buffered frame.
///
/// The session records its stop reason before dropping the relays; without
/// one, the peer detached the link and the relay was removed while the
/// session stayed alive. The link's local state then carries the outcome:
/// `Detached`/`Closed` report a [`LinkStateError::LinkDetached`] with the
/// peer's error from the detach that terminalized the link. Any other state
/// means the relay disappeared without a recorded detach, which is an internal
/// invariant violation (defensive).
pub(crate) fn link_error_from_closed_channel(
    cell: &OnceLock<SessionOutcome>,
    local_state: &LinkState,
) -> LinkStateError {
    if let Some(reason) = cell.get() {
        return LinkStateError::SessionStopped(reason.clone());
    }

    match local_state {
        LinkState::Detached(remote_error) => LinkStateError::LinkDetached(LinkOutcome::Detached {
            remote_error: remote_error.clone(),
        }),
        LinkState::Closed(remote_error) => LinkStateError::LinkDetached(LinkOutcome::Closed {
            remote_error: remote_error.clone(),
        }),
        _ => {
            warn_unrecorded_stop_reason();
            LinkStateError::InvariantViolation
        }
    }
}

/// What a caller can do with a link after an operation failed.
///
/// Returned by the `recovery()` method on [`LinkStateError`], [`SendError`]
/// and [`RecvError`]. The action is derived from the error alone; defensive
/// errors that can cover several link states are classified conservatively.
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
    /// indeterminate (the defensive `IllegalState`/`InvariantViolation`/
    /// `UnexpectedFrame` variants): create a new link.
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

/// Error associated with detaching a link.
///
/// This is a type alias of [`LinkStateError`]. A peer-initiated detach or close
/// is reported as a [`LinkOutcome`] outcome instead of an error; this alias
/// remains only for compatibility with existing signatures.
pub type DetachError = LinkStateError;

/// How the peer detached the link.
///
/// This is an outcome, not a local failure: the peer either suspended the
/// link with a non-closing detach or destroyed it with a closing detach. The
/// `error` field the peer attached to its detach, if any, is carried in
/// `remote_error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkOutcome {
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

impl LinkOutcome {
    /// Whether the peer destroyed the link (`true`) or suspended it (`false`).
    pub fn is_closed(&self) -> bool {
        matches!(self, LinkOutcome::Closed { .. })
    }

    /// The error the peer attached to its detach, if any.
    pub fn remote_error(&self) -> Option<&definitions::Error> {
        match self {
            LinkOutcome::Detached { remote_error } | LinkOutcome::Closed { remote_error } => {
                remote_error.as_ref()
            }
        }
    }
}

/// Error from recording a peer-initiated detach with
/// [`LinkDetach::apply_remote_detach_outcome`].
///
/// The outcome itself is reported as [`LinkOutcome`]; this error means the
/// outcome was *not* recorded because the link was not in a state that can
/// record it, and nothing changed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ApplyRemoteDetachError {
    /// The link was never attached
    #[error("The link is not attached")]
    NotAttached,

    /// The link was already suspended by a previous detach
    #[error("The link is already detached")]
    AlreadyDetached(Option<definitions::Error>),

    /// The link was already closed by a previous detach
    #[error("The link is already closed")]
    AlreadyClosed(Option<definitions::Error>),
}

impl From<ApplyRemoteDetachError> for LinkStateError {
    fn from(value: ApplyRemoteDetachError) -> Self {
        match value {
            ApplyRemoteDetachError::NotAttached => LinkStateError::InvariantViolation,
            ApplyRemoteDetachError::AlreadyDetached(remote_error) => {
                LinkStateError::LinkDetached(LinkOutcome::Detached { remote_error })
            }
            ApplyRemoteDetachError::AlreadyClosed(remote_error) => {
                LinkStateError::LinkDetached(LinkOutcome::Closed { remote_error })
            }
        }
    }
}

/// Failure of a sender transfer.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TransferError {
    /// A local link-state failure
    #[error(transparent)]
    LinkState(#[from] LinkStateError),

    /// The link endpoint has no local handle, i.e. the link is not attached
    #[error("The link is not attached")]
    NotAttached,

    /// The performative could not be serialized
    #[error(transparent)]
    MessageEncodeError(#[from] MessageEncodeError),

    /// The negotiated max frame size cannot fit even the serialized transfer
    /// performative, so the delivery cannot be split into frames
    #[error("The negotiated max frame size is too small for the transfer performative")]
    FrameSizeTooSmall,

    /// The peer requested a transactional acquisition, which is not
    /// implemented
    #[cfg(feature = "transaction")]
    #[error("Transactional acquisition is not implemented")]
    AcquisitionNotImplemented,

    /// The peer detached the link before the transfer completed
    #[error("The peer detached the link")]
    LinkDetached(LinkOutcome),

    /// A frame other than the expected detach arrived while the transfer
    /// waited for link credit
    #[error("Unexpected frame while expecting the peer's detach")]
    UnexpectedFrame,
}

impl From<TransferError> for SendError {
    fn from(value: TransferError) -> Self {
        match value {
            TransferError::LinkState(error) => error.into(),
            TransferError::NotAttached => SendError::NotAttached,
            TransferError::MessageEncodeError(error) => SendError::MessageEncodeError(error),
            TransferError::FrameSizeTooSmall => SendError::FrameSizeTooSmall,
            #[cfg(feature = "transaction")]
            TransferError::AcquisitionNotImplemented => SendError::AcquisitionNotImplemented,
            TransferError::LinkDetached(status) => SendError::LinkDetached(status),
            TransferError::UnexpectedFrame => SendError::UnexpectedFrame,
        }
    }
}

/// Failure delivered through a delivery's settlement channel.
#[derive(Debug, Clone)]
pub(crate) enum DeliveryFailure {
    /// A local link-state failure
    LinkState(LinkStateError),
    /// The peer detached the link while the delivery was pending
    LinkDetached(LinkOutcome),
}

impl From<DeliveryFailure> for SendError {
    fn from(value: DeliveryFailure) -> Self {
        match value {
            DeliveryFailure::LinkState(error) => error.into(),
            DeliveryFailure::LinkDetached(status) => SendError::LinkDetached(status),
        }
    }
}

/// Errors associated with attaching a link as sender
#[derive(Debug, thiserror::Error)]
pub enum SenderAttachError {
    /// The session (or its connection) stopped before the attach completed
    #[error("The session stopped before the link was attached: {:?}", .0)]
    SessionStopped(SessionOutcome),

    /// The session is not in the `Mapped` state (e.g. not begun, or ending)
    #[error("The session is not in a state that permits link attachment")]
    SessionNotMapped,

    /// Link name duplicated
    #[error("Link name is not unique.")]
    DuplicatedLinkName,

    /// Illegal link state
    #[error("Illegal link state")]
    IllegalState,

    /// An internal invariant was violated; this indicates a bug in the library
    #[error("An internal invariant was violated")]
    InvariantViolation,

    /// The local terminus is expecting an Attach from the remote peer
    #[error("Expecting an Attach frame but received a non-Attach frame")]
    NonAttachFrameReceived,

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

    /// The peer's attach reply carried an unsettled map during a
    /// client-initiated attach
    #[error("The peer's attach response carried an unsettled map")]
    UnexpectedUnsettledMap,
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
    LinkStateError(LinkStateError),

    /// The peer detached the link before the delivery was settled
    #[error("The peer detached the link: {:?}", .0)]
    LinkDetached(LinkOutcome),

    /// The link endpoint has no local handle, i.e. the link is not attached
    #[error("The link is not attached")]
    NotAttached,

    /// The negotiated max frame size cannot fit even the serialized transfer
    /// performative, so the message cannot be sent. `max-frame-size` is
    /// negotiated per connection (AMQP 1.0 §2.4.1), so only a new connection
    /// can change it.
    #[error("The negotiated max frame size is too small for the transfer performative")]
    FrameSizeTooSmall,

    /// The peer requested a transactional acquisition, which is not
    /// implemented
    #[error("Transactional acquisition is not implemented")]
    AcquisitionNotImplemented,

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
    #[error(transparent)]
    MessageEncodeError(#[from] MessageEncodeError),

    /// A frame other than the expected detach arrived while the transfer
    /// waited for link credit
    #[error("Unexpected frame while expecting the peer's detach")]
    UnexpectedFrame,
}

impl From<serde_amqp::Error> for SendError {
    fn from(source: serde_amqp::Error) -> Self {
        Self::MessageEncodeError(MessageEncodeError { source })
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
    SessionStopped(SessionOutcome),

    /// The session is not in the `Mapped` state (e.g. not begun, or ending)
    #[error("The session is not in a state that permits link attachment")]
    SessionNotMapped,

    /// Link name is already in use
    #[error("Link name is already in use")]
    DuplicatedLinkName,

    /// Illegal link state
    #[error("Illegal link state")]
    IllegalState,

    /// An internal invariant was violated; this indicates a bug in the library
    #[error("An internal invariant was violated")]
    InvariantViolation,

    /// The local terminus is expecting an Attach from the remote peer
    #[error("Expecting an Attach frame but received a non-Attach frame")]
    NonAttachFrameReceived,

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

    /// The peer's attach reply carried an unsettled map during a
    /// client-initiated attach
    #[error("The peer's attach response carried an unsettled map")]
    UnexpectedUnsettledMap,
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
            ReceiverAttachError::UnexpectedUnsettledMap => AmqpError::IllegalState.into(),
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

impl<'a> TryFrom<&'a SenderAttachError> for definitions::Error {
    type Error = &'a SenderAttachError;

    fn try_from(value: &'a SenderAttachError) -> Result<Self, Self::Error> {
        let condition: ErrorCondition = match value {
            SenderAttachError::SessionStopped(_) => AmqpError::IllegalState.into(),
            SenderAttachError::DuplicatedLinkName => SessionError::HandleInUse.into(),
            SenderAttachError::IllegalState => AmqpError::IllegalState.into(),
            SenderAttachError::NonAttachFrameReceived => AmqpError::NotAllowed.into(),
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
            SenderAttachError::UnexpectedUnsettledMap => AmqpError::IllegalState.into(),

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
    /// The peer sent a frame that is not permitted in the current state
    /// (`amqp:illegal-state`)
    #[error("Illegal local state")]
    IllegalState,

    /// An internal invariant was violated; this indicates a bug in the library
    #[error("An internal invariant was violated")]
    InvariantViolation,

    /// The link already reached a terminal outcome (`Detached`/`Closed`)
    #[error("The link is already detached or closed: {:?}", .0)]
    LinkDetached(LinkOutcome),

    /// The session (or its connection) stopped before the link was detached or closed
    #[error("The session stopped before the link was detached or closed: {:?}", .0)]
    SessionStopped(SessionOutcome),
}

/// Errors associated with receiving a transfer
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReceiverTransferError {
    /// The link endpoint has no local handle, i.e. the link is not attached
    #[error("The link is not attached")]
    NotAttached,

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

/// Error encoding message
#[derive(Debug)]
pub struct MessageEncodeError {
    /// Source error
    pub source: serde_amqp::Error,
}

impl std::fmt::Display for MessageEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error encoding message: {}", self.source)
    }
}

impl std::error::Error for MessageEncodeError {}

/// Errors associated with receiving
#[derive(Debug, thiserror::Error)]
pub enum RecvError {
    /// Errors found in link state
    #[error("Local error: {:?}", .0)]
    LinkStateError(LinkStateError),

    /// The peer detached the link before a delivery could be received
    #[error("The peer detached the link: {:?}", .0)]
    LinkDetached(LinkOutcome),

    /// The link endpoint has no local handle, i.e. the link is not attached
    #[error("The link is not attached")]
    NotAttached,

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

    /// Transactional acquisition is not supported yet
    #[error("Transactional acquisition is not implemented")]
    AcquisitionNotImplemented,
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
            ReceiverTransferError::NotAttached => RecvError::NotAttached,
        }
    }
}

/// The recovery action for a peer detach: a suspended link can be resumed, a
/// destroyed one cannot.
fn link_outcome_recovery(status: &LinkOutcome) -> ErrorRecovery {
    if status.is_closed() {
        ErrorRecovery::NewLink
    } else {
        ErrorRecovery::ReattachLink
    }
}

impl LinkStateError {
    /// Classifies what the caller can do with the link after this error.
    pub fn recovery(&self) -> ErrorRecovery {
        match self {
            Self::IllegalState | Self::InvariantViolation | Self::LinkDetached(_) => {
                ErrorRecovery::NewLink
            }
            Self::SessionStopped(reason) => session_stop_recovery(reason),
        }
    }
}

impl From<LinkStateError> for SendError {
    fn from(value: LinkStateError) -> Self {
        match value {
            LinkStateError::LinkDetached(status) => SendError::LinkDetached(status),
            other => SendError::LinkStateError(other),
        }
    }
}

impl SendError {
    /// Classifies what the caller can do with the link after this error.
    pub fn recovery(&self) -> ErrorRecovery {
        match self {
            Self::LinkStateError(error) => error.recovery(),
            Self::LinkDetached(status) => link_outcome_recovery(status),
            Self::NonTerminalDeliveryState
            | Self::IllegalDeliveryState
            | Self::MessageSizeExceeded(_)
            | Self::MessageEncodeError(_) => ErrorRecovery::UseLink,
            Self::NotAttached | Self::AcquisitionNotImplemented | Self::UnexpectedFrame => {
                ErrorRecovery::NewLink
            }
            // `max-frame-size` is negotiated per connection (AMQP 1.0 §2.4.1),
            // so a new link on the same connection inherits the same limit;
            // only a new connection can change it.
            Self::FrameSizeTooSmall => ErrorRecovery::ReconnectConnection,
        }
    }
}

impl RecvError {
    /// Classifies what the caller can do with the link after this error.
    pub fn recovery(&self) -> ErrorRecovery {
        match self {
            Self::LinkStateError(error) => error.recovery(),
            Self::LinkDetached(status) => link_outcome_recovery(status),
            Self::TransferLimitExceeded
            | Self::MessageDecode(_)
            | Self::IllegalRcvSettleModeInTransfer => ErrorRecovery::UseLink,
            Self::DeliveryIdIsNone
            | Self::DeliveryTagIsNone
            | Self::NotAttached
            | Self::MessageSizeExceeded(_)
            | Self::InconsistentFieldInMultiFrameDelivery
            | Self::AcquisitionNotImplemented => ErrorRecovery::NewLink,
        }
    }
}

/// Type alias for disposition error
pub type DispositionError = LinkStateError;

/// Type alias for flow error
pub type FlowError = LinkStateError;

pub(crate) type SendAttachErrorKind = LinkStateError;

/// Deprecated alias for [`LinkStateError`], which `IllegalLinkStateError` was
/// merged into.
#[deprecated(note = "use `LinkStateError` instead")]
pub type IllegalLinkStateError = LinkStateError;

impl From<LinkStateError> for ReceiverAttachError {
    fn from(value: LinkStateError) -> Self {
        match value {
            LinkStateError::IllegalState => ReceiverAttachError::IllegalState,
            LinkStateError::InvariantViolation => ReceiverAttachError::InvariantViolation,
            // Attach paths never propagate a terminal link outcome; the
            // primary attach error is preserved by the caller instead.
            LinkStateError::LinkDetached(_) => ReceiverAttachError::IllegalState,
            LinkStateError::SessionStopped(reason) => ReceiverAttachError::SessionStopped(reason),
        }
    }
}

impl From<LinkStateError> for SenderAttachError {
    fn from(value: LinkStateError) -> Self {
        match value {
            LinkStateError::IllegalState => SenderAttachError::IllegalState,
            LinkStateError::InvariantViolation => SenderAttachError::InvariantViolation,
            // Attach paths never propagate a terminal link outcome; the
            // primary attach error is preserved by the caller instead.
            LinkStateError::LinkDetached(_) => SenderAttachError::IllegalState,
            LinkStateError::SessionStopped(reason) => SenderAttachError::SessionStopped(reason),
        }
    }
}

impl From<LinkStateError> for RecvError {
    fn from(value: LinkStateError) -> Self {
        match value {
            LinkStateError::LinkDetached(status) => RecvError::LinkDetached(status),
            other => RecvError::LinkStateError(other),
        }
    }
}

impl From<ApplyRemoteDetachError> for RecvError {
    fn from(value: ApplyRemoteDetachError) -> Self {
        Self::from(LinkStateError::from(value))
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

    /// The peer detached the link while it was being resumed
    #[error("The peer detached the link while it was being resumed: {:?}", .0)]
    LinkDetached(LinkOutcome),

    /// The link's unsettled map remained incomplete after repeated
    /// suspend/re-attempt rounds (AMQP 1.0 §2.6.13)
    #[error(
        "The link's unsettled map remained incomplete after repeated suspend/re-attempt rounds"
    )]
    IncompleteUnsettled,

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

    /// Error with sending flow or with a link-state operation
    #[error(transparent)]
    FlowError(#[from] LinkStateError),

    /// The peer detached the link while it was being resumed
    #[error("The peer detached the link while it was being resumed: {:?}", .0)]
    LinkDetached(LinkOutcome),

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

    fn message_encode_error() -> MessageEncodeError {
        let source = serde_amqp::from_slice::<String>(&[]).unwrap_err();
        MessageEncodeError { source }
    }

    #[test]
    fn link_state_error_recovery() {
        assert_eq!(
            LinkStateError::IllegalState.recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            LinkStateError::InvariantViolation.recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            LinkStateError::LinkDetached(LinkOutcome::Detached { remote_error: None }).recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            LinkStateError::LinkDetached(LinkOutcome::Closed { remote_error: None }).recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            LinkStateError::SessionStopped(SessionOutcome::Ended).recovery(),
            ErrorRecovery::ReconnectSession
        );
        assert_eq!(
            LinkStateError::SessionStopped(SessionOutcome::ConnectionStopped(
                ConnectionOutcome::Closed
            ))
            .recovery(),
            ErrorRecovery::ReconnectConnection
        );
    }

    #[test]
    fn detach_error_recovery() {
        assert_eq!(
            DetachError::SessionStopped(SessionOutcome::RemoteEnded).recovery(),
            ErrorRecovery::ReconnectSession
        );
        assert_eq!(
            DetachError::SessionStopped(SessionOutcome::ConnectionStopped(
                ConnectionOutcome::RemoteClosed
            ))
            .recovery(),
            ErrorRecovery::ReconnectConnection
        );
        assert_eq!(DetachError::IllegalState.recovery(), ErrorRecovery::NewLink);
    }

    #[test]
    fn send_error_recovery() {
        assert_eq!(
            SendError::LinkStateError(LinkStateError::SessionStopped(SessionOutcome::RemoteEnded))
                .recovery(),
            ErrorRecovery::ReconnectSession
        );
        assert_eq!(
            SendError::LinkDetached(LinkOutcome::Detached {
                remote_error: Some(test_error())
            })
            .recovery(),
            ErrorRecovery::ReattachLink
        );
        assert_eq!(
            SendError::LinkDetached(LinkOutcome::Closed { remote_error: None }).recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            SendError::UnexpectedFrame.recovery(),
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
            SendError::MessageEncodeError(message_encode_error()).recovery(),
            ErrorRecovery::UseLink
        );
        assert_eq!(SendError::NotAttached.recovery(), ErrorRecovery::NewLink);
        assert_eq!(
            SendError::FrameSizeTooSmall.recovery(),
            ErrorRecovery::ReconnectConnection
        );
        assert_eq!(
            SendError::AcquisitionNotImplemented.recovery(),
            ErrorRecovery::NewLink
        );
    }

    #[test]
    fn recv_error_recovery() {
        assert_eq!(
            RecvError::LinkStateError(LinkStateError::SessionStopped(
                SessionOutcome::ConnectionStopped(ConnectionOutcome::Closed)
            ))
            .recovery(),
            ErrorRecovery::ReconnectConnection
        );
        assert_eq!(
            RecvError::LinkDetached(LinkOutcome::Detached { remote_error: None }).recovery(),
            ErrorRecovery::ReattachLink
        );
        assert_eq!(
            RecvError::LinkDetached(LinkOutcome::Detached {
                remote_error: Some(test_error())
            })
            .recovery(),
            ErrorRecovery::ReattachLink
        );
        assert_eq!(
            RecvError::LinkDetached(LinkOutcome::Closed { remote_error: None }).recovery(),
            ErrorRecovery::NewLink
        );
        assert_eq!(
            RecvError::LinkDetached(LinkOutcome::Closed {
                remote_error: Some(test_error())
            })
            .recovery(),
            ErrorRecovery::NewLink
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
        assert_eq!(RecvError::NotAttached.recovery(), ErrorRecovery::NewLink);
        assert_eq!(
            RecvError::AcquisitionNotImplemented.recovery(),
            ErrorRecovery::NewLink
        );
    }

    #[test]
    fn receiver_transfer_error_mapping() {
        assert!(matches!(
            RecvError::from(ReceiverTransferError::NotAttached),
            RecvError::NotAttached
        ));
    }

    #[test]
    fn encode_error_preserves_source() {
        let source = serde_amqp::from_slice::<String>(&[]).unwrap_err();
        let expected = source.to_string();
        let error = SendError::from(source);
        assert!(matches!(error, SendError::MessageEncodeError(_)));
        assert_eq!(
            error.to_string(),
            format!("Error encoding message: {expected}")
        );
    }

    #[test]
    fn link_detached_converts_to_the_direct_variants() {
        let status = LinkOutcome::Detached { remote_error: None };
        assert!(matches!(
            SendError::from(LinkStateError::LinkDetached(status.clone())),
            SendError::LinkDetached(_)
        ));
        assert!(matches!(
            RecvError::from(LinkStateError::LinkDetached(status)),
            RecvError::LinkDetached(_)
        ));
        assert!(matches!(
            SendError::from(LinkStateError::InvariantViolation),
            SendError::LinkStateError(LinkStateError::InvariantViolation)
        ));
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
