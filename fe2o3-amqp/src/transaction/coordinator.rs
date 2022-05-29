use fe2o3_amqp_types::{
    messaging::DeliveryState,
    performatives::{Disposition, Transfer},
    transaction::{Coordinator, Declare, Discharge},
};

use crate::{
    endpoint::LinkFlow,
    link::{receiver::ReceiverInner, role, Link, ReceiverFlowState},
};

pub(crate) type CoordinatorLink =
    Link<role::Receiver, Coordinator, ReceiverFlowState, DeliveryState>;

/// Transaction coordinator
#[derive(Debug)]
pub struct TxnCoordinator {
    pub(crate) inner: ReceiverInner<CoordinatorLink>,
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

    pub(crate) fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Option<Disposition> {
        todo!()
    }

    pub(crate) fn on_incoming_flow(&mut self, flow: LinkFlow) {
        todo!()
    }
}
