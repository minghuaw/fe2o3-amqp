use fe2o3_amqp_types::{transaction::TransactionError};


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