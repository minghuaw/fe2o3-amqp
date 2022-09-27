//! Transaction
//!
//! > For every transactional interaction, one container acts as the transactional resource, and the
//! > other container acts as the transaction controller. The transactional resource performs
//! > transactional work as requested by the transaction controller.
//!
//! # Controller side
//!
//! Please see [`Controller`], [`Transaction`], and [`OwnedTransaction`]
//!
//! # Resource side
//!
//! Accepting incoming control links is currently only supported on the listerner side.
//!
//! By default, the session will not accept remotely initiated control links even with both
//! `"acceptor"` and `"transaction"` features enabled. In order to allow remotely initiated control
//! links and thus allow remotely declared transactions, the user needs to assign a `ControlLinkAcceptor`
//! to a session acceptor.
//!
//! ```rust
//! let session_acceptor = SessionAcceptor::builder()
//!     .control_link_acceptor(ControlLinkAcceptor::default())
//!     .build();
//! ```
//!

use crate::{
    endpoint::ReceiverLink,
    link::{
        delivery::{DeliveryFut, UnsettledMessage},
        DispositionError, FlowError, LinkFrame,
    },
    util::TryConsume,
    Delivery, Receiver, Sendable, Sender,
};
use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use fe2o3_amqp_types::{
    definitions::{self, AmqpError, DeliveryTag, Fields, SequenceNo},
    messaging::{
        message::__private::Serializable, Accepted, DeliveryState, Message, Modified, Outcome,
        Rejected, Released,
    },
    performatives::Transfer,
    primitives::{OrderedMap, Symbol},
    transaction::{Declared, Discharge, TransactionId, TransactionalState},
};

pub(crate) const TXN_ID_KEY: &str = "txn-id";

mod controller;
pub use controller::*;

mod error;
pub use error::*;
use serde::Serialize;
use serde_amqp::{ser::Serializer, Value};

mod acquisition;
pub use acquisition::*;
use tokio::sync::{mpsc::error::TryRecvError, oneshot};
use tracing::instrument;

mod owned;
pub use owned::*;

pub(crate) mod control_link_frame;

#[cfg_attr(docsrs, doc(cfg(feature = "acceptor")))]
#[cfg(feature = "acceptor")]
pub mod coordinator;

#[cfg_attr(docsrs, doc(cfg(feature = "acceptor")))]
#[cfg(feature = "acceptor")]
pub mod frame;

#[cfg_attr(docsrs, doc(cfg(feature = "acceptor")))]
#[cfg(feature = "acceptor")]
pub mod manager;

#[cfg_attr(docsrs, doc(cfg(feature = "acceptor")))]
#[cfg(feature = "acceptor")]
pub mod session;

/// Trait for generics for TxnAcquisition
#[async_trait]
pub trait TransactionDischarge: Sized {
    /// Errors with discharging
    type Error: Send;

    /// Whether the transaction is already discharged
    fn is_discharged(&self) -> bool;

    /// Discharge the transaction
    async fn discharge(&mut self, fail: bool) -> Result<(), Self::Error>;

    /// Rollback the transaction
    ///
    /// This will send a [`Discharge`] with the `fail` field set to true
    ///
    /// If the coordinator is unable to complete the discharge, the coordinator MUST convey the
    /// error to the controller as a transaction-error
    async fn rollback(mut self) -> Result<(), Self::Error> {
        self.discharge(true).await
    }

    /// Commit the transaction
    ///
    /// This will send a [`Discharge`] with the `fail` field set to false.
    ///
    /// If the coordinator is unable to complete the discharge, the coordinator MUST convey the
    /// error to the controller as a transaction-error
    async fn commit(mut self) -> Result<(), Self::Error> {
        self.discharge(false).await
    }
}

/// Retiring a transaction
#[async_trait]
pub trait TransactionalRetirement {
    /// Error with retirement
    type RetireError: Send;

    /// Associate an outcome with a transaction
    ///
    /// The delivery itself need not be associated with the same transaction as the outcome, or
    /// indeed with any transaction at all. However, the delivery MUST NOT be associated with a
    /// different non-discharged transaction than the outcome. If this happens then the control link
    /// MUST be terminated with a transaction-rollback error.
    async fn retire<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
        outcome: Outcome,
    ) -> Result<(), Self::RetireError>
    where
        T: Send + Sync;

    /// Associate an Accepted outcome with a transaction
    async fn accept<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
    ) -> Result<(), Self::RetireError>
    where
        T: Send + Sync,
    {
        let outcome = Outcome::Accepted(Accepted {});
        self.retire(recver, delivery, outcome).await
    }

    /// Associate a Rejected outcome with a transaction
    async fn reject<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::RetireError>
    where
        T: Send + Sync,
    {
        let outcome = Outcome::Rejected(Rejected { error });
        self.retire(recver, delivery, outcome).await
    }

    /// Associate a Released outcome with a transaction
    async fn release<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
    ) -> Result<(), Self::RetireError>
    where
        T: Send + Sync,
    {
        let outcome = Outcome::Released(Released {});
        self.retire(recver, delivery, outcome).await
    }

    /// Associate a Modified outcome with a transaction
    async fn modify<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
        modified: Modified,
    ) -> Result<(), Self::RetireError>
    where
        T: Send + Sync,
    {
        let outcome = Outcome::Modified(modified);
        self.retire(recver, delivery, outcome).await
    }
}

/// Extension trait that also act as a trait bound for TxnAcquisition
pub trait TransactionExt: TransactionDischarge + TransactionalRetirement {
    /// Get the `txn-id` of the transaction
    fn txn_id(&self) -> &TransactionId;
}

/// A transaction scope for the client side
///
/// [`Transaction`] holds a reference to a [`Controller`], which thus allow reusing the same
/// control link for declaring and discharging of multiple transactions. [`OwnedTransaction`]
/// is an alternative that holds the ownership of a control link.
///
/// # Examples
///
/// Please note that only transactional posting has been tested.
///
/// ## Transactional posting
///
/// ```rust
/// let controller = Controller::attach(&mut session, "controller").await.unwrap();
/// let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
///     .await
///     .unwrap();
///
/// // Commit
/// let mut txn = Transaction::declare(&controller, None).await.unwrap();
/// txn.post(&mut sender, "hello").await.unwrap();
/// txn.post(&mut sender, "world").await.unwrap();
/// txn.commit().await.unwrap();
///
/// // Rollback
/// let mut txn = Transaction::declare(&controller, None).await.unwrap();
/// txn.post(&mut sender, "foo").await.unwrap();
/// txn.rollback().await.unwrap();
///
/// controller.close().await.unwrap();
/// sender.close().await.unwrap();
/// ```
///
/// ## Transactional retirement
///
/// ```rust
/// let controller = Controller::attach(&mut session, "controller").await.unwrap();
/// let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
///     .await
///     .unwrap();
///
/// let delivery: Delivery<Value> = receiver.recv().await.unwrap();
///
/// // Transactionally retiring
/// let mut txn = Transaction::declare(&controller, None).await.unwrap();
/// txn.accept(&mut receiver, &delivery).await.unwrap();
/// txn.commit().await.unwrap();
///
/// controller.close().await.unwrap();
/// receiver.close().await.unwrap();
/// ```
///
/// ## Transactional acquisition
///
/// Please note that this is not supported on the resource side yet.
///
/// ```rust
/// let controller = Controller::attach(&mut session, "controller").await.unwrap();
/// let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
///     .await
///     .unwrap();
///
/// // Transactionally retiring
/// let mut txn = Transaction::declare(&controller, None).await.unwrap();
/// let mut txn_acq = txn.acquire(&mut receiver, 2).await.unwrap();
/// let delivery1: Delivery<Value> = txn_acq.recv().await.unwrap();
/// let delivery2: Delivery<Value> = txn_acq.recv().await.unwrap();
/// txn_acq.accept(&delivery1).await.unwrap();
/// txn_acq.accept(&delivery2).await.unwrap();
/// txn_acq.commit().await.unwrap();
///
/// controller.close().await.unwrap();
/// receiver.close().await.unwrap();
/// ```
#[derive(Debug)]
pub struct Transaction<'t> {
    controller: &'t Controller,
    declared: Declared,
    is_discharged: bool,
}

#[async_trait]
impl<'t> TransactionDischarge for Transaction<'t> {
    type Error = ControllerSendError;

    fn is_discharged(&self) -> bool {
        self.is_discharged
    }

    async fn discharge(&mut self, fail: bool) -> Result<(), Self::Error> {
        if !self.is_discharged {
            self.controller
                .discharge(self.declared.txn_id.clone(), fail)
                .await?;
            self.is_discharged = true;
        }
        Ok(())
    }
}

#[async_trait]
impl<'t> TransactionalRetirement for Transaction<'t> {
    type RetireError = DispositionError;

    /// Associate an outcome with a transaction
    ///
    /// The delivery itself need not be associated with the same transaction as the outcome, or
    /// indeed with any transaction at all. However, the delivery MUST NOT be associated with a
    /// different non-discharged transaction than the outcome. If this happens then the control link
    /// MUST be terminated with a transaction-rollback error.
    async fn retire<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
        outcome: Outcome,
    ) -> Result<(), Self::RetireError>
    where
        T: Send + Sync,
    {
        let txn_state = TransactionalState {
            txn_id: self.declared.txn_id.clone(),
            outcome: Some(outcome),
        };
        let state = DeliveryState::TransactionalState(txn_state);
        recver.inner.dispose(delivery, None, state).await
    }
}

impl<'t> TransactionExt for Transaction<'t> {
    fn txn_id(&self) -> &TransactionId {
        &self.declared.txn_id
    }
}

impl<'t> Transaction<'t> {
    /// Declares a transaction with a default controller
    ///
    /// The user needs to supply a name for the underlying control link.
    pub async fn declare(
        controller: &'t Controller,
        global_id: impl Into<Option<TransactionId>>,
    ) -> Result<Transaction<'t>, ControllerSendError> {
        let declared = controller.declare_inner(global_id.into()).await?;
        Ok(Self {
            controller,
            declared,
            is_discharged: false,
        })
    }

    /// Post a transactional work without waiting for the acknowledgement.
    async fn post_batchable<T>(
        &mut self,
        sender: &mut Sender,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<DeliveryFut<Result<Outcome, PostError>>, PostError>
    where
        T: serde::Serialize,
    {
        // If the transaction controller wishes to associate an outgoing transfer with a
        // transaction, it MUST set the state of the transfer with a transactional-state carrying
        // the appropriate transaction identifier

        // Note that if delivery is split across several transfer frames then all frames MUST be
        // explicitly associated with the same transaction.
        let sendable = sendable.into();
        let state = TransactionalState {
            txn_id: self.declared.txn_id.clone(),
            outcome: None,
        };
        let state = DeliveryState::TransactionalState(state);
        let settlement = sender
            .inner
            .send_with_state::<T, PostError>(sendable, Some(state))
            .await?;

        Ok(DeliveryFut::from(settlement))
    }

    /// Post a transactional work
    pub async fn post<T>(
        &mut self,
        sender: &mut Sender,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<Outcome, PostError>
    where
        T: serde::Serialize,
    {
        // If the transaction controller wishes to associate an outgoing transfer with a
        // transaction, it MUST set the state of the transfer with a transactional-state carrying
        // the appropriate transaction identifier

        // Note that if delivery is split across several transfer frames then all frames MUST be
        // explicitly associated with the same transaction.
        let fut = self.post_batchable(sender, sendable).await?;

        // On receiving a non-settled delivery associated with a live transaction, the transactional
        // resource MUST inform the controller of the presumptive terminal outcome before it can
        // successfully discharge the transaction. That is, the resource MUST send a disposition
        // performative which covers the posted transfer with the state of the delivery being a
        // transactional-state with the correct transaction identified, and a terminal outcome. This
        // informs the controller of the outcome that will be in effect at the point that the
        // transaction is successfully discharged.
        fut.await
    }

    /// Acquire a transactional work
    ///
    /// This will send
    pub async fn acquire<'r>(
        self,
        recver: &'r mut Receiver,
        credit: SequenceNo,
    ) -> Result<TxnAcquisition<'r, Transaction<'t>>, FlowError> {
        let value = Value::Binary(self.declared.txn_id.clone());
        {
            let mut writer = recver.inner.link.flow_state.lock.write();
            match &mut writer.properties {
                Some(fields) => {
                    if fields.contains_key(TXN_ID_KEY) {
                        return Err(FlowError::IllegalState);
                    }

                    fields.insert(Symbol::from(TXN_ID_KEY), value);
                }
                None => {
                    let mut fields = Fields::new();
                    fields.insert(Symbol::from(TXN_ID_KEY), value);
                }
            }
        }

        match recver
            .inner
            .link
            .send_flow(&recver.inner.outgoing, Some(credit), None, false)
            .await
        {
            Ok(_) => Ok(TxnAcquisition { txn: self, recver }),
            Err(error) => {
                let mut writer = recver.inner.link.flow_state.lock.write();
                if let Some(fields) = &mut writer.properties {
                    fields.remove(TXN_ID_KEY);
                }
                Err(error)
            }
        }
    }
}

impl<'t> Drop for Transaction<'t> {
    #[instrument]
    fn drop(&mut self) {
        if !self.is_discharged {
            // rollback
            let discharge = Discharge {
                txn_id: self.declared.txn_id.clone(),
                fail: Some(true),
            };
            // As with the declare message, it is an error if the sender sends the transfer pre-settled.
            let message = Message::builder().value(discharge).build();
            let mut payload = BytesMut::new();
            let mut serializer = Serializer::from((&mut payload).writer());
            if let Err(error) = Serializable(message).serialize(&mut serializer) {
                tracing::error!(?error);
                return;
            }
            // let payload = BytesMut::from(payload);
            let payload = payload.freeze();
            let payload_copy = payload.clone();

            let mut inner = self.controller.inner.blocking_lock();

            match inner.link.flow_state.try_consume(1) {
                Ok(_) => {
                    let input_handle = match inner
                        .link
                        .input_handle
                        .clone()
                        .ok_or(AmqpError::IllegalState)
                    {
                        Ok(handle) => handle,
                        Err(error) => {
                            tracing::error!(?error);
                            return;
                        }
                    };
                    let handle = match inner.link.output_handle.clone() {
                        Some(handle) => handle.into(),
                        None => return,
                    };
                    // let tag = self.flow_state.state().delivery_count().await.to_be_bytes();
                    let tag = match inner.link.flow_state.state().lock.try_read() {
                        Some(inner) => inner.delivery_count.to_be_bytes(),
                        None => return,
                    };
                    let delivery_tag = DeliveryTag::from(tag);

                    let transfer = Transfer {
                        handle,
                        delivery_id: None,
                        delivery_tag: Some(delivery_tag.clone()),
                        message_format: Some(0),
                        settled: Some(false),
                        more: false, // This message should be small enough
                        rcv_settle_mode: None,
                        state: None,
                        resume: false,
                        aborted: false,
                        batchable: false,
                    };

                    // try receive in case of detach
                    match inner.incoming.try_recv() {
                        Ok(_) => {
                            // The only frames that are relayed is detach
                            return;
                        }
                        Err(error) => match error {
                            TryRecvError::Empty => {}
                            TryRecvError::Disconnected => return,
                        },
                    }

                    // Send out Rollback
                    let frame = LinkFrame::Transfer {
                        input_handle,
                        performative: transfer,
                        payload,
                    };
                    if inner.outgoing.blocking_send(frame).is_err() {
                        // Channel is already closed
                        return;
                    }

                    // TODO: Wait for accept or not?
                    // The transfer is sent unsettled and will be
                    // inserted into
                    let (tx, rx) = oneshot::channel();
                    let unsettled = UnsettledMessage::new(payload_copy, tx);
                    {
                        let mut guard = match inner.link.unsettled.try_write() {
                            Some(guard) => guard,
                            None => return,
                        };
                        guard
                            .get_or_insert(OrderedMap::new())
                            .insert(delivery_tag, unsettled);
                    }
                    match rx.blocking_recv() {
                        Ok(Some(state)) => match state {
                            DeliveryState::Accepted(_) => {}
                            _ => {
                                tracing::error!(error = ?state);
                            }
                        },
                        Ok(None) => {
                            tracing::error!(error = ?ControllerSendError::IllegalDeliveryState)
                        }
                        Err(error) => {
                            tracing::error!(?error);
                        }
                    };
                }
                Err(error) => {
                    tracing::error!(?error);
                }
            }
        }
    }
}
