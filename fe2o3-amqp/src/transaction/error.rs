use fe2o3_amqp_types::{transaction::TransactionError, messaging::{Rejected, Released, Modified, DeliveryState, Outcome}};

use crate::link::{SenderAttachError, SendError, DetachError, LinkStateError, IllegalLinkStateError, delivery::FromDeliveryState};


/// Errors with allocation of new transacation ID
#[derive(Debug)]
pub enum AllocTxnIdError {
    /// Allocation of transaction ID is not implemented
    /// 
    /// This happens when transaction session is not enabled
    NotImplemented,

    /// Session must have dropped
    InvalidSessionState,
}

/// Errors with discharging a transaction at the transaction manager
#[derive(Debug)]
pub enum DischargeError {
    /// Session must have dropped
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
pub enum CoordinatorError {
    /// The global transaction ID is not implemented yet
    GlobalIdNotImplemented,

    /// Session must have dropped
    InvalidSessionState,

    /// 
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
            AllocTxnIdError::InvalidSessionState => Self::InvalidSessionState,
        }
    }
}

impl From<DischargeError> for CoordinatorError {
    fn from(value: DischargeError) -> Self {
        match value {
            DischargeError::InvalidSessionState => Self::InvalidSessionState,
            // DischargeError::TransactionError(error) => {
            //     let condition = ErrorCondition::TransactionError(error);
            //     let error = definitions::Error::new(condition, None, None);
            //     let rejected = Rejected { error: Some(error) };
            //     Self::Reject(rejected)
            // },
            DischargeError::TransactionError(error) => Self::TransactionError(error)
        }
    }
}

/// Errors with declaring an OwnedTransaction
#[derive(Debug)]
pub enum OwnedDeclareError {
    /// Error with attaching the control link
    AttachError(SenderAttachError),

    /// Error with sending Declare
    SendError(SendError)
}

impl From<SenderAttachError> for OwnedDeclareError {
    fn from(value: SenderAttachError) -> Self {
        Self::AttachError(value)
    }
}

impl From<SendError> for OwnedDeclareError {
    fn from(value: SendError) -> Self {
        Self::SendError(value)
    }
}

/// Errors with discharging an OwnedTransaction
#[derive(Debug)]
pub enum OwnedDischargeError {
    /// Error with sending Discharge
    SendError(SendError),

    /// Error with closing the control link
    DetachError(DetachError),
}

impl From<SendError> for OwnedDischargeError {
    fn from(value: SendError) -> Self {
        Self::SendError(value)
    }
}

impl From<DetachError> for OwnedDischargeError {
    fn from(value: DetachError) -> Self {
        Self::DetachError(value)
    }
}

/// Error associated with sending a txn message
#[derive(Debug, thiserror::Error)]
pub enum PostError {
    /// Errors found in link state
    #[error("Local error: {:?}", .0)]
    LinkStateError(#[from] LinkStateError),

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

impl From<DetachError> for PostError {
    fn from(error: DetachError) -> Self {
        Self::Detached(error)
    }
}

impl From<IllegalLinkStateError> for PostError {
    fn from(value: IllegalLinkStateError) -> Self {
        match value {
            IllegalLinkStateError::IllegalState => LinkStateError::IllegalState.into(),
            IllegalLinkStateError::IllegalSessionState => LinkStateError::IllegalSessionState.into(),
        }
    }
}

type PostResult = Result<(), PostError>;

impl FromDeliveryState for PostResult {
    fn from_delivery_state(state: DeliveryState) -> Self {
        match state {
            DeliveryState::Received(_)
            | DeliveryState::Accepted(_)
            | DeliveryState::Rejected(_)
            | DeliveryState::Released(_)
            | DeliveryState::Modified(_)
            | DeliveryState::Declared(_) => Err(PostError::IllegalDeliveryState),
            DeliveryState::TransactionalState(txn) => match txn.outcome {
                Some(Outcome::Accepted(_)) => Ok(()),
                Some(Outcome::Rejected(value)) => Err(PostError::Rejected(value)),
                Some(Outcome::Released(value)) => Err(PostError::Released(value)),
                Some(Outcome::Modified(value)) => Err(PostError::Modified(value)),
                Some(Outcome::Declared(_)) | None => Err(PostError::IllegalDeliveryState),
            },
        }
    }
}

// /// Errors with handling Post (Transfer) at the resource
// #[derive(Debug)]
// pub enum ResourcePostError {

// }