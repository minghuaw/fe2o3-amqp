use fe2o3_amqp_types::messaging::{Accepted, DeliveryState, Outcome, Rejected};

use crate::link::{
    delivery::{FromDeliveryFailure, FromDeliveryState, FromPreSettled},
    DetachError, DetachStatus, DeliveryFailure, LinkStateError,
    MessageEncodeError, MessageSizeExceeded, SendError, SenderAttachError, SessionStopReason,
    TransferError,
};

/// Errors with allocation of new transacation ID
#[derive(Debug)]
pub(crate) enum CoordinatorAllocTxnIdError {
    /// Allocation of transaction ID is not implemented
    ///
    /// This happens when transaction session is not enabled
    NotImplemented,

    /// Session must have dropped
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "acceptor")]
    InvalidSessionState,
}

cfg_acceptor! {
    use fe2o3_amqp_types::transaction::TransactionError;

    /// Errors with discharging a transaction at the transaction manager
    #[derive(Debug)]
    pub(crate) enum CoordinatorDischargeError {
        /// Session must have dropped
        #[cfg(not(target_arch = "wasm32"))]
        #[cfg(feature = "acceptor")]
        InvalidSessionState,

        /// If the coordinator is unable to complete the discharge, the coordinator MUST convey the error to the controller
        /// as a transaction-error. If the source for the link to the coordinator supports the rejected outcome, then the
        /// message MUST be rejected with this outcome carrying the transaction-error.
        TransactionError(TransactionError),
    }

    impl From<TransactionError> for CoordinatorDischargeError {
        fn from(value: TransactionError) -> Self {
            Self::TransactionError(value)
        }
    }

    /// Errors on the transacitonal resource side
    #[derive(Debug)]
    pub(crate) enum CoordinatorError {
        /// The global transaction ID is not implemented yet
        #[cfg(not(target_arch = "wasm32"))]
        #[cfg(feature = "acceptor")]
        GlobalIdNotImplemented,
    
        /// Session must have dropped
        #[cfg(not(target_arch = "wasm32"))]
        #[cfg(feature = "acceptor")]
        InvalidSessionState,
    
        /// The allocation of transaction ID is not implemented
        AllocTxnIdNotImplemented,
    
        /// If the coordinator is unable to complete the discharge, the coordinator MUST convey the error to the controller
        /// as a transaction-error. If the source for the link to the coordinator supports the rejected outcome, then the
        /// message MUST be rejected with this outcome carrying the transaction-error.
        TransactionError(TransactionError),
    }
    
    impl From<CoordinatorAllocTxnIdError> for CoordinatorError {
        fn from(value: CoordinatorAllocTxnIdError) -> Self {
            match value {
                CoordinatorAllocTxnIdError::NotImplemented => Self::AllocTxnIdNotImplemented,
                #[cfg(not(target_arch = "wasm32"))]
                #[cfg(feature = "acceptor")]
                CoordinatorAllocTxnIdError::InvalidSessionState => Self::InvalidSessionState,
            }
        }
    }
    
    impl From<CoordinatorDischargeError> for CoordinatorError {
        fn from(value: CoordinatorDischargeError) -> Self {
            match value {
                #[cfg(not(target_arch = "wasm32"))]
                #[cfg(feature = "acceptor")]
                CoordinatorDischargeError::InvalidSessionState => Self::InvalidSessionState,
                CoordinatorDischargeError::TransactionError(error) => Self::TransactionError(error),
            }
        }
    }
}

/// Errors with sending message on the control link
#[derive(Debug, thiserror::Error)]
pub enum ControllerSendError {
    /// Errors found in link state
    #[error("Local error: {:?}", .0)]
    LinkStateError(LinkStateError),

    /// The peer detached the link before the delivery was settled
    #[error("The peer detached the link: {:?}", .0)]
    LinkDetached(DetachStatus),

    /// The link endpoint has no local handle, i.e. the link is not attached
    #[error("The link is not attached")]
    NotAttached,

    /// The negotiated max frame size cannot fit even the serialized transfer
    /// performative, so the message cannot be sent
    #[error("The negotiated max frame size is too small for the transfer performative")]
    FrameSizeTooSmall,

    /// The peer requested a transactional acquisition, which is not
    /// implemented
    #[error("Transactional acquisition is not implemented")]
    AcquisitionNotImplemented,

    /// The message was rejected
    #[error("Outcome Rejected: {:?}", .0)]
    Rejected(Rejected),

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

impl From<SendError> for ControllerSendError {
    fn from(value: SendError) -> Self {
        match value {
            SendError::LinkStateError(state) => Self::LinkStateError(state),
            SendError::LinkDetached(status) => Self::LinkDetached(status),
            SendError::NotAttached => Self::NotAttached,
            SendError::FrameSizeTooSmall => Self::FrameSizeTooSmall,
            SendError::AcquisitionNotImplemented => Self::AcquisitionNotImplemented,
            SendError::NonTerminalDeliveryState => Self::NonTerminalDeliveryState,
            SendError::IllegalDeliveryState => Self::IllegalDeliveryState,
            SendError::MessageSizeExceeded(error) => Self::MessageSizeExceeded(error),
            SendError::MessageEncodeError(error) => Self::MessageEncodeError(error),
            SendError::UnexpectedFrame => Self::UnexpectedFrame,
        }
    }
}

impl From<DeliveryFailure> for ControllerSendError {
    fn from(value: DeliveryFailure) -> Self {
        match value {
            DeliveryFailure::LinkState(error) => error.into(),
            DeliveryFailure::LinkDetached(status) => Self::LinkDetached(status),
        }
    }
}

impl From<LinkStateError> for ControllerSendError {
    fn from(value: LinkStateError) -> Self {
        match value {
            LinkStateError::LinkDetached(status) => Self::LinkDetached(status),
            other => Self::LinkStateError(other),
        }
    }
}

/// Error declaring a transaction on the control link
pub type DeclareError = ControllerSendError;

/// Error discharging a transaction on the control link
pub type DischargeError = ControllerSendError;

/// Errors with declaring an OwnedTransaction
#[derive(Debug, thiserror::Error)]
pub enum OwnedDeclareError {
    /// Error with attaching the control link
    #[error(transparent)]
    AttachError(SenderAttachError),

    /// Error with sending Declare
    #[error(transparent)]
    ControllerSendError(ControllerSendError),
}

impl From<SenderAttachError> for OwnedDeclareError {
    fn from(value: SenderAttachError) -> Self {
        Self::AttachError(value)
    }
}

impl From<ControllerSendError> for OwnedDeclareError {
    fn from(value: ControllerSendError) -> Self {
        Self::ControllerSendError(value)
    }
}

/// Errors with discharging an OwnedTransaction
#[derive(Debug, thiserror::Error)]
pub enum OwnedDischargeError {
    /// Error with sending Discharge
    #[error(transparent)]
    ControllerSendError(ControllerSendError),

    /// Error with closing the control link
    #[error(transparent)]
    DetachError(DetachError),
}

impl From<ControllerSendError> for OwnedDischargeError {
    fn from(value: ControllerSendError) -> Self {
        Self::ControllerSendError(value)
    }
}

impl From<LinkStateError> for OwnedDischargeError {
    fn from(value: LinkStateError) -> Self {
        Self::DetachError(value)
    }
}

/// Error associated with sending a txn message
///
/// It is similar to [`SendError`] but differs in how transactional states
/// are interpreted
#[derive(Debug, thiserror::Error)]
pub enum PostError {
    /// Errors found in link state
    #[error("Local error: {:?}", .0)]
    LinkStateError(LinkStateError),

    /// The peer detached the link before the delivery was settled
    #[error("The peer detached the link: {:?}", .0)]
    LinkDetached(DetachStatus),

    /// The link endpoint has no local handle, i.e. the link is not attached
    #[error("The link is not attached")]
    NotAttached,

    /// The negotiated max frame size cannot fit even the serialized transfer
    /// performative, so the message cannot be sent
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

impl From<serde_amqp::Error> for PostError {
    fn from(source: serde_amqp::Error) -> Self {
        Self::MessageEncodeError(MessageEncodeError { source })
    }
}

impl From<LinkStateError> for PostError {
    fn from(value: LinkStateError) -> Self {
        match value {
            LinkStateError::LinkDetached(status) => Self::LinkDetached(status),
            other => Self::LinkStateError(other),
        }
    }
}

impl From<MessageSizeExceeded> for PostError {
    fn from(error: MessageSizeExceeded) -> Self {
        Self::MessageSizeExceeded(error)
    }
}

impl From<TransferError> for PostError {
    fn from(value: TransferError) -> Self {
        match value {
            TransferError::LinkState(error) => error.into(),
            TransferError::NotAttached => Self::NotAttached,
            TransferError::MessageEncodeError(error) => Self::MessageEncodeError(error),
            TransferError::FrameSizeTooSmall => Self::FrameSizeTooSmall,
            TransferError::AcquisitionNotImplemented => Self::AcquisitionNotImplemented,
            TransferError::LinkDetached(status) => Self::LinkDetached(status),
            TransferError::UnexpectedFrame => Self::UnexpectedFrame,
        }
    }
}

type PostResult = Result<Outcome, PostError>;

impl FromDeliveryState for PostResult {
    fn from_none() -> Self {
        Err(PostError::IllegalDeliveryState)
    }

    fn from_delivery_state(state: DeliveryState) -> Self {
        match state {
            DeliveryState::Received(_)
            | DeliveryState::Accepted(_)
            | DeliveryState::Rejected(_)
            | DeliveryState::Released(_)
            | DeliveryState::Modified(_)
            | DeliveryState::Declared(_) => Err(PostError::IllegalDeliveryState),
            DeliveryState::TransactionalState(txn) => match txn.outcome {
                Some(Outcome::Accepted(value)) => Ok(Outcome::Accepted(value)),
                Some(Outcome::Rejected(value)) => Ok(Outcome::Rejected(value)),
                Some(Outcome::Released(value)) => Ok(Outcome::Released(value)),
                Some(Outcome::Modified(value)) => Ok(Outcome::Modified(value)),
                Some(Outcome::Declared(_)) | None => Err(PostError::IllegalDeliveryState),
            },
        }
    }
}

impl FromPreSettled for PostResult {
    fn from_settled() -> Self {
        Ok(Outcome::Accepted(Accepted {}))
    }
}

impl FromDeliveryFailure for PostResult {
    fn from_oneshot_recv_error(_: tokio::sync::oneshot::error::RecvError) -> Self {
        // The session relay and the link endpoint fail the pending deliveries
        // before they drop their maps, so this is defensive only.
        Err(PostError::LinkStateError(LinkStateError::InvariantViolation))
    }

    fn from_session_stop_reason(reason: SessionStopReason) -> Self {
        Err(PostError::LinkStateError(LinkStateError::SessionStopped(reason)))
    }

    fn from_link_state_error(error: LinkStateError) -> Self {
        Err(error.into())
    }

    fn from_detach_status(status: DetachStatus) -> Self {
        Err(PostError::LinkDetached(status))
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp_types::definitions;

    use super::{DetachStatus, FromDeliveryFailure, LinkStateError, PostError, PostResult};

    #[test]
    fn test_post_result_from_link_state_error() {
        let result =
            <PostResult as FromDeliveryFailure>::from_link_state_error(LinkStateError::IllegalState);
        match result {
            Err(PostError::LinkStateError(LinkStateError::IllegalState)) => {}
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_post_result_from_detach_status() {
        let result = <PostResult as FromDeliveryFailure>::from_detach_status(
            DetachStatus::Closed { remote_error: None },
        );
        match result {
            Err(PostError::LinkDetached(DetachStatus::Closed { remote_error: None })) => {}
            other => panic!("unexpected result: {:?}", other),
        }

        let error = definitions::Error::new(
            definitions::ConnectionError::ConnectionForced,
            Some("remote closed".to_string()),
            None,
        );
        let result = <PostResult as FromDeliveryFailure>::from_detach_status(DetachStatus::Closed {
            remote_error: Some(error.clone()),
        });
        match result {
            Err(PostError::LinkDetached(DetachStatus::Closed {
                remote_error: Some(actual),
            })) => {
                assert_eq!(actual, error);
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }
}
