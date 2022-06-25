
/// The transaction manager was unable to allocate new transaction IDs
#[derive(Debug)]
pub enum TransactionManagerError {
    /// The transaction manager failed to allocate a new transaction ID
    AllocateTxnIdFailed,
}

