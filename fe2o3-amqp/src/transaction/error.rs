use fe2o3_amqp_types::messaging::{Accepted, DeliveryState, Outcome, Rejected};

use crate::link::{
    delivery::{FromDeliveryState, FromOneshotRecvError, FromPreSettled},
    DetachError, IllegalLinkStateError, LinkStateError, SendError, SenderAttachError,
};

/// Errors with allocation of new transacation ID
#[derive(Debug)]
pub(crate) enum AllocTxnIdError {
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
    pub(crate) enum DischargeError {
        /// Session must have dropped
        #[cfg(not(target_arch = "wasm32"))]
        #[cfg(feature = "acceptor")]
        InvalidSessionState,

        /// If the coordinator is unable to complete the discharge, the coordinator MUST convey the error to the controller
        /// as a transaction-error. If the source for the link to the coordinator supports the rejected outcome, then the
        /// message MUST be rejected with this outcome carrying the transaction-error.
        TransactionError(TransactionError),
    }

    impl From<TransactionError> for DischargeError {
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
    
    impl From<AllocTxnIdError> for CoordinatorError {
        fn from(value: AllocTxnIdError) -> Self {
            match value {
                AllocTxnIdError::NotImplemented => Self::AllocTxnIdNotImplemented,
                #[cfg(not(target_arch = "wasm32"))]
                #[cfg(feature = "acceptor")]
                AllocTxnIdError::InvalidSessionState => Self::InvalidSessionState,
            }
        }
    }
    
    impl From<DischargeError> for CoordinatorError {
        fn from(value: DischargeError) -> Self {
            match value {
                #[cfg(not(target_arch = "wasm32"))]
                #[cfg(feature = "acceptor")]
                DischargeError::InvalidSessionState => Self::InvalidSessionState,
                DischargeError::TransactionError(error) => Self::TransactionError(error),
            }
        }
    }
}

/// Errors with sending message on the control link
#[derive(Debug, thiserror::Error)]
pub enum ControllerSendError {
    /// Errors found in link state
    #[error("Local error: {:?}", .0)]
    LinkStateError(#[from] LinkStateError),

    /// The remote peer detached with error
    #[error("Link is detached {:?}", .0)]
    Detached(DetachError),

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

    /// Error serializing message
    #[error("Error encoding message")]
    MessageEncodeError,
}

impl From<SendError> for ControllerSendError {
    fn from(value: SendError) -> Self {
        match value {
            SendError::LinkStateError(state) => Self::LinkStateError(state),
            SendError::Detached(value) => Self::Detached(value),
            SendError::NonTerminalDeliveryState => Self::NonTerminalDeliveryState,
            SendError::IllegalDeliveryState => Self::IllegalDeliveryState,
            SendError::MessageEncodeError => Self::MessageEncodeError,
        }
    }
}

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

impl From<DetachError> for OwnedDischargeError {
    fn from(value: DetachError) -> Self {
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

impl From<serde_amqp::Error> for PostError {
    fn from(_: serde_amqp::Error) -> Self {
        Self::MessageEncodeError
    }
}

impl From<DetachError> for PostError {
    fn from(error: DetachError) -> Self {
        Self::Detached(error)
    }
}

impl From<IllegalLinkStateError> for PostError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => LinkStateError::IllegalState.into(),
            IllegalLinkStateError::IllegalSessionState => {
                LinkStateError::IllegalSessionState.into()
            }
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

impl FromOneshotRecvError for PostResult {
    fn from_oneshot_recv_error(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Err(PostError::LinkStateError(
            LinkStateError::IllegalSessionState,
        ))
    }
}
