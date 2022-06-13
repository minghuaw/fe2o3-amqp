//! Manages incoming transaction on the resource side

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use fe2o3_amqp_types::{transaction::TransactionId, performatives::{Attach, Detach, Transfer, Disposition, Flow}};
use tokio::sync::mpsc;

use crate::{endpoint::{self, IncomingChannel}, link::{LinkFrame, AttachError}, Session};

use super::frame::TransactionalWork;

#[async_trait]
pub(crate) trait HandleControlLink {
    type Error: Send;

    async fn on_incoming_control_attach(&mut self, channel: IncomingChannel, attach: Attach) -> Result<(), Self::Error>;

    fn on_incoming_control_detach(&mut self, channel: IncomingChannel, detach: Detach) -> Result<(), Self::Error>;
}

/// How an incoming transaction should be handled in a session
#[cfg(feature = "transaction")]
pub(crate) trait ResourceTxnManager {
    type Error: Send;

    fn allocate_transaction_id(&mut self) -> Result<TransactionId, Self::Error>;

    fn commit_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error>;

    fn rollback_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error>;

    fn on_incoming_txn_posting(&mut self, transfer: Transfer) -> Result<(), Self::Error>;

    fn on_incoming_txn_retirement(&mut self, disposition: Disposition) -> Result<(), Self::Error>;

    fn on_incoming_txn_acquisition(&mut self, flow: Flow) -> Result<(), Self::Error>;
}

/// Transaction manager
#[derive(Debug)]
pub(crate) struct TransactionManager {
    pub control_link_tx: mpsc::Sender<LinkFrame>,
    pub txn_id_source: u64,
    pub txns: BTreeMap<TransactionId, TransactionalWork>,
    pub coordinators: BTreeSet<()>
}

impl TransactionManager {

}

/// 
#[derive(Debug)]
pub(crate) struct TxnSession<S> where S: endpoint::Session {
    session: S,
    txn_manager: TransactionManager,
}

#[async_trait]
impl HandleControlLink for TxnSession<Session> {
    type Error = AttachError;

    async fn on_incoming_control_attach(&mut self, channel: IncomingChannel, attach: Attach) -> Result<(), Self::Error> {
        todo!()
    }

    fn on_incoming_control_detach(&mut self, channel: IncomingChannel, detach: Detach) -> Result<(), Self::Error> {
        todo!()
    }
}