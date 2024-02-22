use std::future::Future;

use fe2o3_amqp_types::{
    messaging::Accepted,
    transaction::{TransactionError, TransactionId},
};

use crate::transaction::AllocTxnIdError;

use super::Session;

pub(crate) trait HandleDeclare: Session {
    fn allocate_transaction_id(&mut self) -> Result<TransactionId, AllocTxnIdError>;
}


pub(crate) trait HandleDischarge: Session {
    fn commit_transaction(
        &mut self,
        txn_id: TransactionId,
    ) -> impl Future<Output = Result<Result<Accepted, TransactionError>, Self::Error>> + Send;
    fn rollback_transaction(
        &mut self,
        txn_id: TransactionId,
    ) -> Result<Result<Accepted, TransactionError>, Self::Error>;
}
