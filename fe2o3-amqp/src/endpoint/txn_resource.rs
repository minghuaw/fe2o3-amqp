use async_trait::async_trait;
use fe2o3_amqp_types::{transaction::{TransactionId, TransactionError}, messaging::Accepted};
use tokio::sync::mpsc;

use crate::transaction::{TransactionManagerError, AllocTxnIdError, manager::ResourceTransaction, frame::TxnWorkFrame};

use super::Session;

pub(crate) trait HandleDeclare: Session {
    fn allocate_transaction_id(&mut self, work_frame_tx: mpsc::Sender<TxnWorkFrame>) -> Result<TransactionId, AllocTxnIdError>;
}

#[async_trait]
pub(crate) trait HandleDischarge: Session {
    async fn commit_transaction(&mut self, txn: ResourceTransaction) -> Result<Result<Accepted, TransactionError>, Self::Error>;
    async fn rollback_transaction(&mut self, txn_id: TransactionId) -> Result<Result<Accepted, TransactionError>, Self::Error>;
}
