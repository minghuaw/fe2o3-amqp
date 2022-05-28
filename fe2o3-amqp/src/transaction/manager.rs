//! Listener side transaction manager

use std::collections::BTreeMap;

use fe2o3_amqp_types::{performatives::{Attach, Transfer, Disposition, Flow, Detach}, transaction::{Declare, Discharge, TransactionId}};

use crate::{endpoint::{LinkFlow, InputHandle}, link::receiver::ReceiverInner};

use super::TxnCoordinator;
/// Transaction manager
#[derive(Debug)]
pub struct TxnManager {
    pub(crate) coordinators: BTreeMap<InputHandle, TxnCoordinator>
}

impl TxnManager {
    pub(crate) fn on_incoming_attach(&mut self, attach: Attach) {


        todo!()
    }

    pub(crate) fn on_incoming_transfer(&mut self, transfer: Transfer) -> Option<Transfer> {
        todo!()
    }

    pub(crate) fn on_incoming_disposition(&mut self, disposition: Disposition) -> Option<Disposition> {
        todo!()
    }

    pub(crate) fn on_incoming_flow(&mut self, flow: LinkFlow) -> Option<Flow> {
        todo!()
    }

    pub(crate) fn on_incoming_detach(&mut self, detach: Detach) {
        todo!()
    }
}
