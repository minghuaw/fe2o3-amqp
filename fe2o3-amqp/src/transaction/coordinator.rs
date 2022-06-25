//! Control link coordinator

use fe2o3_amqp_types::{
    definitions::{self, AmqpError},
    messaging::{Accepted, DeliveryState},
    performatives::Attach,
    transaction::{Coordinator, Declare, Declared, Discharge, TxnCapability},
};
use tokio::sync::mpsc;
use tracing::{instrument, trace};

use crate::{
    acceptor::{link::SharedLinkAcceptorFields, local_receiver_link::LocalReceiverLinkAcceptor},
    control::SessionControl,
    link::{receiver::ReceiverInner, role, Link, LinkFrame, ReceiverFlowState, ReceiverAttachError, RecvError},
    Delivery,
};

use super::control_link_frame::ControlMessageBody;

pub(crate) type CoordinatorLink =
    Link<role::Receiver, Coordinator, ReceiverFlowState, DeliveryState>;

/// An acceptor that handles incoming control links
#[derive(Debug, Clone)]
pub struct ControlLinkAcceptor {
    shared: SharedLinkAcceptorFields,
    inner: LocalReceiverLinkAcceptor<TxnCapability>,
}

impl Default for ControlLinkAcceptor {
    fn default() -> Self {
        Self {
            shared: Default::default(),
            inner: Default::default(),
        }
    }
}

impl ControlLinkAcceptor {
    #[instrument(skip_all)]
    pub(crate) async fn accept_incoming_attach(
        &self,
        remote_attach: Attach,
        control: &mpsc::Sender<SessionControl>,
        outgoing: &mpsc::Sender<LinkFrame>,
    ) -> Result<TxnCoordinator, ReceiverAttachError> {
        trace!(control_link_attach = ?remote_attach);
        self
            .inner
            .accept_incoming_attach_inner(&self.shared, remote_attach, control, outgoing)
            .await
            .map(|inner| TxnCoordinator { inner })
    }
}

/// Transaction coordinator
#[derive(Debug)]
pub(crate) struct TxnCoordinator {
    inner: ReceiverInner<CoordinatorLink>,
}

impl TxnCoordinator {
    async fn on_declare(&mut self, declare: &Declare) -> Result<Declared, ()> {
        todo!()
    }

    async fn on_discharge(&mut self, discharge: &Discharge) -> Result<Accepted, ()> {
        todo!()
    }

    async fn on_delivery(&mut self, delivery: Delivery<ControlMessageBody>) {
        
        let body = match delivery.body() {
            fe2o3_amqp_types::messaging::Body::Value(v) => &v.0,
            fe2o3_amqp_types::messaging::Body::Sequence(_)
            | fe2o3_amqp_types::messaging::Body::Data(_) => {
                // Message Decode Error?
                todo!()
            }
        };

        let result = match body {
            ControlMessageBody::Declare(declare) => {
                self.on_declare(declare).await.map(DeliveryState::Declared)
            }
            ControlMessageBody::Discharge(discharge) => self
                .on_discharge(discharge)
                .await
                .map(DeliveryState::Accepted),
        };

        let delivery_state = match result {
            Ok(state) => state,
            Err(error) => todo!(),
        };

        if let Err(_) = self
            .inner
            .dispose(delivery.delivery_id, delivery.delivery_tag, delivery_state)
            .await
        {
            todo!()
        }

        todo!()
    }

    pub async fn event_loop(mut self) {
        tracing::info!("Coordinator started");
        loop {
            let delivery: Delivery<ControlMessageBody> = match self.inner.recv().await {
                Ok(d) => d,
                Err(error) => {
                    // TODO: How should error be handled?
                    tracing::error!(?error);

                    match error {
                        RecvError::LinkStateError(_) => todo!(),
                        RecvError::TransferLimitExceeded => todo!(),
                        RecvError::DeliveryIdIsNone => todo!(),
                        RecvError::DeliveryTagIsNone => todo!(),
                        RecvError::MessageDecodeError => todo!(),
                        RecvError::IllegalRcvSettleModeInTransfer => todo!(),
                        RecvError::InconsistentFieldInMultiFrameDelivery => todo!(),
                    }

                    break;
                }
            };

            self.on_delivery(delivery).await
        }
    }
}
