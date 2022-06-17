//! Control link coordinator

use fe2o3_amqp_types::{
    messaging::DeliveryState,
    performatives::Attach,
    transaction::{Coordinator, TxnCapability},
};
use tokio::sync::mpsc;

use crate::{
    acceptor::{
        link::SharedLinkAcceptorFields, local_receiver_link::LocalReceiverLinkAcceptor,
        LinkAcceptor,
    },
    control::SessionControl,
    link::{receiver::ReceiverInner, role, AttachError, Link, LinkFrame, ReceiverFlowState},
};

pub(crate) type CoordinatorLink =
    Link<role::Receiver, Coordinator, ReceiverFlowState, DeliveryState>;

/// An acceptor that handles incoming control links
#[derive(Debug, Clone)]
pub(crate) struct ControlLinkAcceptor {
    shared: SharedLinkAcceptorFields,
    inner: LocalReceiverLinkAcceptor<TxnCapability>,
}

impl ControlLinkAcceptor {
    pub async fn accept_incoming_attach(
        &self,
        remote_attach: Attach,
        control: mpsc::Sender<SessionControl>,
        session_tx: mpsc::Sender<LinkFrame>,
    ) -> Result<TxnCoordinator, AttachError> {
        self.inner
            .accept_incoming_attach_inner(&self.shared, remote_attach, &control, &session_tx)
            .await
            .map(|inner| TxnCoordinator { inner })
    }
}

/// Transaction coordinator
#[derive(Debug)]
pub(crate) struct TxnCoordinator {
    inner: ReceiverInner<CoordinatorLink>,
}
