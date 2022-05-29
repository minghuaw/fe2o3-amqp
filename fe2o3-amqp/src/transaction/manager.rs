//! Listener side transaction manager

use std::{collections::BTreeMap, marker::PhantomData};

use fe2o3_amqp_types::{
    definitions::{ReceiverSettleMode, SenderSettleMode, AmqpError, Role, self},
    messaging::DeliveryState,
    performatives::{Attach, Detach, Disposition, Flow, Transfer},
    transaction::{Coordinator, Declare, Discharge, TransactionId, TxnCapability},
};

use crate::{
    acceptor::LinkAcceptor,
    endpoint::{InputHandle, LinkFlow},
    link::{self, receiver::ReceiverInner, role, state::LinkState, AttachError, ReceiverFlowState},
    session::SessionHandle,
};

use super::TxnCoordinator;
/// Transaction manager
#[derive(Debug)]
pub struct TxnManager {
    pub(crate) acceptor: LinkAcceptor,
    pub(crate) coordinators: BTreeMap<TransactionId, InputHandle>,
}

impl TxnManager {
    /// Creates a builder for TxnManager
    pub fn builder() {
        todo!()
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
            Role::Sender => self.acceptor.accept_as_new_receiver_inner::<R, Coordinator>(remote_attach, session).await?,
            Role::Receiver => {
                self.acceptor.reject_incoming_attach(remote_attach, session).await?;
                return Err(AttachError::Local(definitions::Error::new(
                    AmqpError::NotAllowed,
                    "Controller has to be a sender".to_string(),
                    None
                )))
            }
        };
        let coordinator = TxnCoordinator { inner };
        todo!()
    }

    pub(crate) fn on_incoming_transfer(&mut self, transfer: Transfer) -> Option<Transfer> {
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

