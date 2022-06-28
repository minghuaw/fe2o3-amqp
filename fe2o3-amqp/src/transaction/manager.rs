//! Manages incoming transaction on the resource side

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use fe2o3_amqp_types::{
    performatives::{Attach, Disposition, Flow, Transfer},
    transaction::{TransactionId, TransactionError}, messaging::Accepted,
};
use tokio::sync::mpsc;

use crate::{
    endpoint::{IncomingChannel, LinkFlow, HandleDischarge},
    link::LinkFrame,
    session::frame::SessionFrame,
    Payload,
};

use super::{coordinator::ControlLinkAcceptor, frame::TxnWorkFrame};

#[async_trait]
pub(crate) trait HandleControlLink {
    type Error: Send;

    async fn on_incoming_control_attach(
        &mut self,
        attach: Attach,
    ) -> Result<(), Self::Error>;
}

/// How an incoming transaction should be handled in a session
#[async_trait]
pub(crate) trait HandleTransactionalWork {
    type Error: Send;

    async fn on_incoming_txn_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<(), Self::Error>;

    async fn on_incoming_txn_flow(
        &mut self,
        flow: Flow,
    ) -> Result<(), Self::Error>;

    async fn on_incoming_txn_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<(), Self::Error>;

    fn on_outgoing_txn_transfer(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error>;

    fn on_outgoing_txn_flow(&mut self, flow: LinkFlow) -> Result<SessionFrame, Self::Error>;

    fn on_outgoing_txn_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error>;
}

/// Transaction manager
#[derive(Debug)]
pub(crate) struct TransactionManager {
    pub control_link_outgoing: mpsc::Sender<LinkFrame>,
    pub txns: BTreeMap<TransactionId, ResourceTransaction>,
    pub control_link_acceptor: Arc<ControlLinkAcceptor>,
}

impl TransactionManager {
    pub(crate) fn new(
        control_link_outgoing: mpsc::Sender<LinkFrame>,
        control_link_acceptor: ControlLinkAcceptor,
    ) -> Self {
        Self {
            control_link_outgoing,
            txns: BTreeMap::new(),
            control_link_acceptor: Arc::new(control_link_acceptor),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ResourceTransaction {
    pub frames: Vec<TxnWorkFrame>,
}

impl ResourceTransaction {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp_types::transaction::TransactionId;
    use uuid::Uuid;

    #[test]
    fn test_recover_key_from_txn_id() {
        let uuid = Uuid::new_v4();
        let txn_id = TransactionId::from(uuid.clone().into_bytes());
        let uuid2 = Uuid::from_slice(txn_id.as_ref()).unwrap();
        assert_eq!(uuid, uuid2);
    }
}
