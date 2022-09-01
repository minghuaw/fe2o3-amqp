//! Manages incoming transaction on the resource side

use std::sync::Arc;

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::Role,
    messaging::{Accepted, DeliveryState, Outcome},
    performatives::{Attach, Disposition, Transfer},
    primitives::OrderedMap,
    transaction::{TransactionId, TransactionalState},
};
use tokio::sync::mpsc;

use crate::{link::LinkFrame, Payload};

use super::{coordinator::ControlLinkAcceptor, frame::TxnWorkFrame};

#[async_trait]
pub(crate) trait HandleControlLink {
    type Error: Send;

    async fn on_incoming_control_attach(&mut self, attach: Attach) -> Result<(), Self::Error>;
}

/// Transaction manager
#[derive(Debug)]
pub(crate) struct TransactionManager {
    pub control_link_outgoing: mpsc::Sender<LinkFrame>,
    pub txns: OrderedMap<TransactionId, ResourceTransaction>,
    pub control_link_acceptor: Arc<ControlLinkAcceptor>,
}

impl TransactionManager {
    pub(crate) fn new(
        control_link_outgoing: mpsc::Sender<LinkFrame>,
        control_link_acceptor: ControlLinkAcceptor,
    ) -> Self {
        Self {
            control_link_outgoing,
            txns: OrderedMap::new(),
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

    pub(crate) fn on_incoming_post(
        &mut self,
        txn_id: TransactionId,
        transfer: Transfer,
        payload: Payload,
    ) -> Option<Disposition> {
        let disposition = match transfer.settled {
            Some(true) => None,
            Some(false) | None => {
                // On receiving a non-settled delivery associated with a live transaction, the transactional
                // resource MUST inform the controller of the presumptive terminal outcome before it can
                // successfully discharge the transaction. That is, the resource MUST send a disposition
                // performative which covers the posted transfer with the state of the delivery being a
                // transactional-state with the correct transaction identified, and a terminal outcome. This
                // informs the controller of the outcome that will be in effect at the point that the
                // transaction is successfully discharged.
                transfer.delivery_id.map(|delivery_id| {
                    let txn_state = TransactionalState {
                        txn_id: txn_id.clone(),
                        outcome: Some(Outcome::Accepted(Accepted {})),
                    };

                    Disposition {
                        role: Role::Receiver,
                        first: delivery_id,
                        last: None,
                        settled: true,
                        state: Some(DeliveryState::TransactionalState(txn_state)),
                        batchable: false,
                    }
                })
            }
        };

        let frame = TxnWorkFrame::Post { transfer, payload };
        self.frames.push(frame);

        disposition
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
