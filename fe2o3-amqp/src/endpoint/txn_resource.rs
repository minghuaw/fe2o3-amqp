use fe2o3_amqp_types::transaction::TransactionId;

use crate::transaction::TransactionManagerError;

use super::Session;

pub(crate) trait HandleDeclare: Session {
    fn allocate_transaction_id(&mut self) -> Result<TransactionId, TransactionManagerError>;
}

pub(crate) trait HandleDischarge: Session {
    fn commit_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error>;

    fn rollback_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error>;
}