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

// /// State of transaction coordinator
// pub enum TxnCoordinatorState {
//     /// The control link is established
//     Established,

//     /// 
// }

pub(crate) mod coordinator_state {
    pub struct Established {}

    pub struct TxnDeclared {}

    pub struct Closed {}
}

/// Transaction coordinator
#[derive(Debug)]
pub struct TxnCoordinator<T> {
    pub(crate) inner: ReceiverInner<CoordinatorLink>,
    pub(crate) state: T
}

impl<T> TxnCoordinator<T> {
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

    async fn even_loop(self) {

    }
}
