//! Control link coordinator

use fe2o3_amqp_types::{transaction::{Coordinator, TxnCapability}, messaging::DeliveryState, performatives::Attach};
use tokio::sync::mpsc;

use crate::{link::{receiver::ReceiverInner, Link, role, ReceiverFlowState, LinkFrame, AttachError}, acceptor::{LinkAcceptor, local_receiver_link::LocalReceiverLinkAcceptor, link::SharedLinkAcceptorFields}, control::SessionControl};

pub(crate) type CoordinatorLink = Link<role::Receiver, Coordinator, ReceiverFlowState, DeliveryState>;

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
    ) -> Result<Coordinator, AttachError> {
        todo!()
    }
}


/// Transaction coordinator
#[derive(Debug)]
pub(crate) struct TxnCoordinator {
    inner: ReceiverInner<CoordinatorLink>
}