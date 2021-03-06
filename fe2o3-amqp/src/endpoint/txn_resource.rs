use async_trait::async_trait;
use fe2o3_amqp_types::{
    messaging::Accepted,
    transaction::{TransactionError, TransactionId},
};

use crate::transaction::AllocTxnIdError;

use super::Session;

pub(crate) trait HandleDeclare: Session {
    fn allocate_transaction_id(&mut self) -> Result<TransactionId, AllocTxnIdError>;
}

#[async_trait]
pub(crate) trait HandleDischarge: Session {
    async fn commit_transaction(
        &mut self,
        txn_id: TransactionId,
    ) -> Result<Result<Accepted, TransactionError>, Self::Error>;
    fn rollback_transaction(
        &mut self,
        txn_id: TransactionId,
    ) -> Result<Result<Accepted, TransactionError>, Self::Error>;
}
