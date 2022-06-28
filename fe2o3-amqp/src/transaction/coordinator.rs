//! Control link coordinator

use std::collections::{HashMap};

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, DeliveryNumber, DeliveryTag, LinkError, ReceiverSettleMode},
    messaging::{Accepted, DeliveryState, Rejected},
    performatives::Attach,
    transaction::{
        Coordinator, Declare, Declared, Discharge, TransactionError, TransactionId, TxnCapability,
    },
};
use tokio::sync::mpsc;
use tracing::instrument;

use crate::{
    acceptor::{link::SharedLinkAcceptorFields, local_receiver_link::LocalReceiverLinkAcceptor, SupportedReceiverSettleModes},
    control::SessionControl,
    link::{
        receiver::ReceiverInner,
        role,
        shared_inner::{LinkEndpointInner, LinkEndpointInnerDetach},
        IllegalLinkStateError, Link, LinkFrame, ReceiverAttachError, ReceiverFlowState, RecvError,
    },
    util::Running,
    Delivery,
};

use super::{
    control_link_frame::ControlMessageBody, frame::TxnWorkFrame, manager::ResourceTransaction,
    CoordinatorError,
};

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
            inner: LocalReceiverLinkAcceptor {
                supported_rcv_settle_modes: SupportedReceiverSettleModes::Second,
                fallback_rcv_settle_mode: ReceiverSettleMode::Second,
                credit_mode: Default::default(),
                target_capabilities: None,
            },
        }
    }
}

impl ControlLinkAcceptor {
    #[instrument(skip_all)]
    pub(crate) async fn accept_incoming_attach(
        &self,
        remote_attach: Attach,
        control: mpsc::Sender<SessionControl>,
        outgoing: mpsc::Sender<LinkFrame>,
    ) -> Result<TxnCoordinator, ReceiverAttachError> {
        tracing::info!(control_link_attach = ?remote_attach);
        let (work_frame_tx, work_frame_rx) = mpsc::channel(self.shared.buffer_size);
        self.inner
            .accept_incoming_attach_inner(&self.shared, remote_attach, control, outgoing)
            .await
            .map(|inner| TxnCoordinator {
                inner,
                txns: HashMap::new(),
                work_frame_rx,
                work_frame_tx,
            })
    }
}

enum SuccessfulOutcome {
    Declared(Declared),
    Accepted(Accepted),
}

impl From<SuccessfulOutcome> for DeliveryState {
    fn from(value: SuccessfulOutcome) -> Self {
        match value {
            SuccessfulOutcome::Declared(declared) => DeliveryState::Declared(declared),
            SuccessfulOutcome::Accepted(accepted) => DeliveryState::Accepted(accepted),
        }
    }
}

/// Transaction coordinator
#[derive(Debug)]
pub(crate) struct TxnCoordinator {
    inner: ReceiverInner<CoordinatorLink>,
    txns: HashMap<TransactionId, ResourceTransaction>,
    work_frame_rx: mpsc::Receiver<TxnWorkFrame>,
    work_frame_tx: mpsc::Sender<TxnWorkFrame>,
}

impl TxnCoordinator {
    async fn on_declare(&mut self, declare: &Declare) -> Result<Declared, CoordinatorError> {
        match declare.global_id {
            Some(_) => Err(CoordinatorError::GlobalIdNotImplemented),
            None => {
                let txn_id = super::session::allocate_transaction_id(
                    self.inner.session_control(),
                    self.work_frame_tx.clone(),
                )
                .await?;

                // The TxnManager has the authoratitive version of all active txns, so
                // the txn-id obtained from the TxnManager should be "guaranteed" to be unique
                self.txns
                    .insert(txn_id.clone(), ResourceTransaction::new(txn_id.clone()));
                Ok(Declared { txn_id })
            }
        }
    }

    async fn on_discharge(&mut self, discharge: &Discharge) -> Result<Accepted, CoordinatorError> {
        let txn = match self.txns.remove(&discharge.txn_id) {
            Some(txn) => txn,
            None => {
                return Err(CoordinatorError::TransactionError(
                    TransactionError::UnknownId,
                ))
            }
        };

        match discharge.fail {
            Some(true) => {
                super::session::rollback_transaction(self.inner.session_control(), txn.txn_id)
                    .await
                    .map_err(Into::into)
            }
            Some(false) | None => {
                // The fail field is treated as a false if unset in AmqpNetLite
                super::session::commit_transaction(self.inner.session_control(), txn)
                    .await
                    .map_err(Into::into)
            }
        }
    }

    #[instrument(skip_all)]
    async fn on_delivery(&mut self, delivery: Delivery<ControlMessageBody>) -> Running {
        let body = match delivery.body() {
            fe2o3_amqp_types::messaging::Body::Value(v) => &v.0,
            fe2o3_amqp_types::messaging::Body::Sequence(_)
            | fe2o3_amqp_types::messaging::Body::Data(_) => {
                // Message Decode Error?
                let error = definitions::Error::new(
                    AmqpError::DecodeError,
                    "The coordinator is only expecting Declare or Discharge frames".to_string(),
                    None,
                );
                // TODO: detach instead of closing
                let _ = self.inner.close_with_error(Some(error)).await;
                return Running::Stop;
            }
        };

        tracing::debug!(body = ?delivery.body());

        let result = match body {
            ControlMessageBody::Declare(declare) => self
                .on_declare(declare)
                .await
                .map(SuccessfulOutcome::Declared),
            ControlMessageBody::Discharge(discharge) => self
                .on_discharge(discharge)
                .await
                .map(SuccessfulOutcome::Accepted),
        };

        self.handle_delivery_result(delivery.delivery_id, delivery.delivery_tag, result)
            .await
    }

    #[instrument(skip_all)]
    async fn on_recv_error(&mut self, error: RecvError) -> Running {
        match error {
            RecvError::LinkStateError(error) => match error {
                crate::link::LinkStateError::IllegalState => {
                    tracing::error!(?error);
                    let error = definitions::Error::new(AmqpError::IllegalState, None, None);
                    // TODO: detach instead of closing
                    let _ = self.inner.close_with_error(Some(error)).await;
                    Running::Stop
                }
                crate::link::LinkStateError::IllegalSessionState => {
                    tracing::error!(?error);
                    // Session must have already stopped
                    Running::Stop
                }
                crate::link::LinkStateError::ExpectImmediateDetach => {
                    tracing::error!(?error);
                    let _ = self.inner.close_with_error(None).await;
                    // TODO: detach instead of closing
                    Running::Stop
                }
                crate::link::LinkStateError::RemoteDetached
                | crate::link::LinkStateError::RemoteClosed
                | crate::link::LinkStateError::RemoteDetachedWithError(_)
                | crate::link::LinkStateError::RemoteClosedWithError(_) => {
                    self.inner
                        .close_with_error(None)
                        .await
                        .unwrap_or_else(|err| tracing::error!(detach_error = ?err));
                    Running::Stop
                }
            },
            RecvError::TransferLimitExceeded => {
                tracing::error!(?error);
                let error = definitions::Error::new(LinkError::TransferLimitExceeded, None, None);
                // TODO: detach instead of closing
                let _ = self.inner.close_with_error(Some(error)).await;
                Running::Stop
            }
            RecvError::DeliveryIdIsNone
            | RecvError::DeliveryTagIsNone
            | RecvError::MessageDecodeError
            | RecvError::IllegalRcvSettleModeInTransfer
            | RecvError::InconsistentFieldInMultiFrameDelivery => {
                tracing::error!(?error);
                let error =
                    definitions::Error::new(AmqpError::NotAllowed, format!("{:?}", error), None);
                // TODO: detach instead of closing
                let _ = self.inner.close_with_error(Some(error)).await;
                Running::Stop
            }
        }
    }

    async fn handle_delivery_result(
        &mut self,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        result: Result<SuccessfulOutcome, CoordinatorError>,
    ) -> Running {
        let disposition_result = match result {
            Ok(outcome) => {
                self.inner
                    .dispose(delivery_id, delivery_tag, Some(true), outcome.into())
                    .await
            }
            Err(error) => match error {
                CoordinatorError::GlobalIdNotImplemented => {
                    let error = TransactionError::UnknownId;
                    let description = "Global transaction ID is not implemented".to_string();
                    self.reject(delivery_id, delivery_tag, error, description)
                        .await
                }
                CoordinatorError::InvalidSessionState => {
                    // Session must have dropped
                    return Running::Stop;
                }
                CoordinatorError::AllocTxnIdNotImplemented => {
                    let error = TransactionError::UnknownId;
                    let description =
                        "Allocation of new transaction ID is not implemented".to_string();
                    self.reject(delivery_id, delivery_tag, error, description)
                        .await
                }
                CoordinatorError::TransactionError(error) => {
                    self.reject(delivery_id, delivery_tag, error, None).await
                }
            },
        };

        match disposition_result {
            Ok(_) => Running::Continue,
            Err(disposition_error) => match disposition_error {
                IllegalLinkStateError::IllegalState => {
                    let error = definitions::Error::new(AmqpError::IllegalState, None, None);
                    // TODO: detach instead of closing
                    let _ = self.inner.close_with_error(Some(error)).await;
                    Running::Stop
                }
                IllegalLinkStateError::IllegalSessionState => {
                    // Session must have already dropped
                    Running::Stop
                }
            },
        }
    }

    async fn reject(
        &mut self,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        error: TransactionError,
        description: impl Into<Option<String>>,
    ) -> Result<(), IllegalLinkStateError> {
        // let condition = ErrorCondition::TransactionError(error);
        let error = definitions::Error::new(error, description, None);
        let state = DeliveryState::Rejected(Rejected { error: Some(error) });

        self.inner.dispose(delivery_id, delivery_tag, Some(true), state).await
    }

    fn on_txn_work_frame(&mut self, work_frame: TxnWorkFrame) -> Result<Running, TransactionError> {
        match work_frame
            .txn_id()
            .and_then(|txn_id| self.txns.get_mut(txn_id))
        {
            Some(txn) => {
                txn.frames.push(work_frame);
                Ok(Running::Continue)
            }
            None => Err(TransactionError::UnknownId),
        }
    }

    #[instrument(skip_all)]
    pub async fn event_loop(mut self) {
        tracing::info!("Coordinator started");
        loop {
            let running = tokio::select! {
                delivery = self.inner.recv() => {
                    match delivery {
                        Ok(delivery) => self.on_delivery(delivery).await,
                        Err(error) => self.on_recv_error(error).await,
                    }
                },
                work_frame = self.work_frame_rx.recv() => {
                    tracing::info!(?work_frame);
                    match work_frame {
                        Some(work_frame) => {
                            match self.on_txn_work_frame(work_frame) {
                                Ok(running) => running,
                                Err(error) => {
                                    // Detach because work frame could be Transfer, Flow, or Disposition, and only
                                    // Transfer can be rejected
                                    let error = definitions::Error::new(error, None, None);
                                    let _ = self.inner.close_with_error(Some(error)).await;
                                    Running::Stop
                                },
                            }
                        },
                        None => Running::Stop,
                    }
                }
            };

            if let Running::Stop = running {
                break;
            }
        }
    }
}

impl Drop for TxnCoordinator {
    fn drop(&mut self) {
        for (txn_id, _) in self.txns.drain() {
            if let Err(_) = self
                .inner
                .session_control()
                .try_send(SessionControl::AbortTransaction(txn_id))
            {
                // Session must have dropped
                return;
            }
        }
    }
}
