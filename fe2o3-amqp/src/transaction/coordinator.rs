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
    link::{receiver::ReceiverInner, role, AttachError, Link, LinkFrame, ReceiverFlowState},
    Delivery,
};

use super::control_link_msg::ControlMessageBody;

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
    ) -> Result<TxnCoordinator, AttachError> {
        trace!(control_link_attach = ?remote_attach);
        match self
            .inner
            .accept_incoming_attach_inner(&self.shared, remote_attach, control, outgoing)
            .await
        {
            Ok(inner) => Ok(TxnCoordinator { inner }),
            Err((error, remote_attach)) => Err(crate::acceptor::link::handle_attach_error(
                error,
                remote_attach,
                outgoing,
                control,
            )
            .await),
        }
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

    pub async fn event_loop(mut self) {
        tracing::info!("Coordinator started");
        loop {
            let delivery: Delivery<ControlMessageBody> = match self.inner.recv().await {
                Ok(d) => d,
                Err(error) => {
                    // TODO: How should error be handled?
                    tracing::error!(?error);

                    match error {
                        crate::link::Error::Local(err) => {
                            let _ = self.inner.close_with_error(Some(err)).await;
                        }
                        crate::link::Error::Detached(_) => {
                            let _ = self.inner.close_with_error(None).await;
                        }
                    }

                    break;
                }
            };

            let body = match delivery.body() {
                fe2o3_amqp_types::messaging::Body::Value(v) => &v.0,
                fe2o3_amqp_types::messaging::Body::Sequence(_)
                | fe2o3_amqp_types::messaging::Body::Data(_) => {
                    let error = definitions::Error::new(
                        AmqpError::NotAllowed,
                        "The coordinator is expecting either a Declare or Discharge message body"
                            .to_string(),
                        None,
                    );
                    let _ = self.inner.close_with_error(Some(error)).await;
                    break;
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
        }
    }
}
