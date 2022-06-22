use fe2o3_amqp_types::transaction::TransactionId;

pub(crate) trait HandleDeclare {
    type Error: Send;

    fn allocate_transaction_id(&mut self) -> Result<TransactionId, Self::Error>;
}

pub(crate) trait HandleDischarge {
    type Error: Send;
    
    fn commit_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error>;

    fn rollback_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error>;
}