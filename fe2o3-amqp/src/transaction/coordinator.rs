use fe2o3_amqp_types::{transaction::{Coordinator, Declare, Discharge}, messaging::DeliveryState, performatives::{Transfer, Disposition}};

use crate::{link::{ReceiverFlowState, Link, role, receiver::ReceiverInner}, endpoint::LinkFlow};


pub(crate) type CoordinatorLink = Link<role::Receiver, Coordinator, ReceiverFlowState, DeliveryState>;

/// Transaction coordinator
#[derive(Debug)]
pub struct TxnCoordinator {
    inner: ReceiverInner<CoordinatorLink>,
}

impl TxnCoordinator {
    pub(crate) fn on_incoming_transfer(&mut self, transfer: Transfer) -> Option<Transfer> {
        todo!()
    }

    pub(crate) fn on_declare(&mut self, declare: Declare) {
        todo!()
    }

    pub(crate) fn on_discharge(&mut self, discharge: Discharge) {
        todo!()
    }

    pub(crate) fn on_incoming_disposition(&mut self, disposition: Disposition) -> Option<Disposition> {
        todo!()
    }

    pub(crate) fn on_incoming_flow(&mut self, flow: LinkFlow) {
        todo!()
    }
}