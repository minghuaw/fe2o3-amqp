use async_trait::async_trait;
use fe2o3_amqp_types::{transaction::{TransactionId, TransactionError}, messaging::Accepted};

use crate::transaction::{TransactionManagerError, AllocTxnIdError};

use super::Session;

pub(crate) trait HandleDeclare {
    fn allocate_transaction_id(&mut self) -> Result<TransactionId, AllocTxnIdError>;
}

#[async_trait]
pub(crate) trait HandleDischarge {
    async fn commit_transaction(&mut self, txn_id: TransactionId) -> Result<Accepted, TransactionError>;
    async fn rollback_transaction(&mut self, txn_id: TransactionId) -> Result<Accepted, TransactionError>;
}
