//! Listener side transaction manager

use std::{
    collections::BTreeMap,
    marker::PhantomData,
    sync::{atomic::AtomicU64, Arc},
};

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, ReceiverSettleMode, Role, SenderSettleMode},
    messaging::DeliveryState,
    performatives::{Attach, Detach, Disposition, Flow, Transfer},
    transaction::{Coordinator, Declare, Discharge, TransactionId, TxnCapability},
};
use tokio::sync::mpsc;

use crate::{
    acceptor::LinkAcceptor,
    endpoint::{InputHandle, LinkFlow},
    link::{self, receiver::ReceiverInner, role, state::LinkState, AttachError, ReceiverFlowState},
    session::SessionHandle,
    Payload,
};

use super::TxnCoordinator;

#[derive(Debug)]
pub(crate) enum TxnWorkFrame {
    Post {
        input_handle: InputHandle,
        transfer: Transfer,
        payload: Payload,
    },
    Retire(Disposition),
    Acquire(),
}

pub(crate) type TxnWorkSender = mpsc::Sender<TxnWorkFrame>;

/// Transaction manager
#[derive(Debug)]
pub struct ResourceManager {
    pub(crate) acceptor: LinkAcceptor,
    pub(crate) resources: BTreeMap<TransactionId, TxnWorkSender>,
    pub(crate) txn_id_source: Arc<AtomicU64>,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self {
            acceptor: Default::default(),
            resources: Default::default(),
            txn_id_source: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl ResourceManager {
    /// Creates a new transaction resource manager
    pub fn new(acceptor: LinkAcceptor) -> Self {
        Self {
            acceptor,
            resources: Default::default(),
            txn_id_source: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(crate) async fn accept_incoming_attach<R>(
        &mut self,
        remote_attach: Attach,
        session: &mut SessionHandle<R>,
    ) -> Result<(), AttachError> {
        let remote_attach = self
            .acceptor
            .reject_if_source_or_target_is_none(remote_attach, session)
            .await?;

        let inner = match remote_attach.role {
            Role::Sender => {
                self.acceptor
                    .accept_as_new_receiver_inner::<R, Coordinator>(remote_attach, session)
                    .await?
            }
            Role::Receiver => {
                self.acceptor
                    .reject_incoming_attach(remote_attach, session)
                    .await?;
                return Err(AttachError::Local(definitions::Error::new(
                    AmqpError::NotAllowed,
                    "Controller has to be a sender".to_string(),
                    None,
                )));
            }
        };
        let coordinator = TxnCoordinator {
            inner,
        };
        todo!()
    }

    pub(crate) fn intercept_incoming_transfer(&mut self, transfer: Transfer) -> Option<Transfer> {
        todo!()
    }

    pub(crate) fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Option<Disposition> {
        todo!()
    }

    pub(crate) fn on_incoming_flow(&mut self, flow: LinkFlow) -> Option<Flow> {
        todo!()
    }

    pub(crate) fn on_incoming_detach(&mut self, detach: Detach) {
        todo!()
    }
}
