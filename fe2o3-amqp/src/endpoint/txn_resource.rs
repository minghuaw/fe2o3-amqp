use async_trait::async_trait;
use fe2o3_amqp_types::transaction::TransactionId;

use crate::transaction::TransactionManagerError;

use super::Session;

pub(crate) trait HandleDeclare: Session {
    fn allocate_transaction_id(&mut self) -> Result<TransactionId, TransactionManagerError>;
}

#[async_trait]
pub(crate) trait HandleDischarge: Session {
    async fn commit_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error>;
    async fn rollback_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error>;
}
