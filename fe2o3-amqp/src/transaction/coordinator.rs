//! Control link coordinator

use fe2o3_amqp_types::{transaction::Coordinator, messaging::DeliveryState, performatives::Attach};
use tokio::sync::mpsc;

use crate::{link::{receiver::ReceiverInner, Link, role, ReceiverFlowState, LinkFrame, AttachError}, acceptor::LinkAcceptor, control::SessionControl};

pub(crate) type CoordinatorLink = Link<role::Receiver, Coordinator, ReceiverFlowState, DeliveryState>;

/// An acceptor that handles incoming control links
#[derive(Debug, Clone)]
pub(crate) struct ControlLinkAcceptor {
    inner: LinkAcceptor,
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