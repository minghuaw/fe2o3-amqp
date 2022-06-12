use fe2o3_amqp_types::{transaction::TransactionId, performatives::{Attach, Detach, Transfer, Disposition, Flow}};

use crate::{endpoint::TransactionSession, Session, transaction::AllocateTxnIdFailed};

impl TransactionSession for Session {
    fn allocate_transaction_id(&mut self) -> Result<TransactionId, AllocateTxnIdFailed> {
        self.txn_manager.allocate_transaction_id()
    }

    fn commit_transaction(&mut self, txn_id: TransactionId) -> Result<(), ()> {
        todo!()
    }

    fn rollback_transaction(&mut self, txn_id: TransactionId) -> Result<(), ()> {
        todo!()
    }

    fn on_incoming_control_attach(&mut self, remote_attach: Attach) {
        if remote_attach.source.is_none() || remote_attach.target.is_none() {
            
        }
        
        
        todo!()
    }

    fn on_incoming_control_detach(&mut self, detach: Detach) {
        todo!()
    }

    fn on_incoming_txn_posting(&mut self, transfer: Transfer) {
        todo!()
    }

    fn on_incoming_txn_retirement(&mut self, disposition: Disposition) {
        todo!()
    }

    fn on_incoming_txn_acquisition(&mut self, flow: Flow) {
        todo!()
    }
}