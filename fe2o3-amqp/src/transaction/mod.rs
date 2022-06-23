//! Transaction

use crate::{
    endpoint::{ReceiverLink, Settlement},
    link::{self, LinkFrame, delivery::UnsettledMessage},
    util::TryConsume,
    Delivery, Receiver, Sendable, Sender,
};
use bytes::{BufMut, BytesMut};
use fe2o3_amqp_types::{
    definitions::{self, AmqpError, Fields, SequenceNo, DeliveryTag},
    messaging::{
        message::__private::Serializable, Accepted, DeliveryState, Message, Modified, Outcome,
        Rejected, Released,
    },
    performatives::Transfer,
    primitives::Symbol,
    transaction::{
        Declared, Discharge, TransactionId, TransactionalState, 
    },
};

mod controller;
pub use controller::*;

mod error;
pub use error::*;
use serde::Serialize;
use serde_amqp::{ser::Serializer, to_value};

mod acquisition;
pub use acquisition::*;
use tokio::sync::{oneshot, mpsc::error::TryRecvError};
use tracing::instrument;

pub(crate) mod control_link_msg;

#[cfg(feature = "acceptor")]
pub mod coordinator;

pub mod frame;

#[cfg(feature = "acceptor")]
pub mod manager;

#[cfg(feature = "acceptor")]
pub mod session;

const TXN_ID_KEY: &str = "txn-id";

/// A transaction scope for the client side
///
/// # Examples
///
/// Please note that only transactional posting has been tested.
///
/// ## Transactional posting
///
/// ```rust
/// let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
///     .await
///     .unwrap();
///
/// // Commit
/// let mut txn = Transaction::declare(&mut session, "controller-1", None)
///     .await
///     .unwrap();
/// txn.post(&mut sender, "hello").await.unwrap();
/// txn.post(&mut sender, "world").await.unwrap();
/// txn.commit().await.unwrap();
///
/// // Rollback
/// let mut txn = Transaction::declare(&mut session, "controller-2", None)
///     .await
///     .unwrap();
/// txn.post(&mut sender, "foo").await.unwrap();
/// txn.rollback().await.unwrap();
/// ```
///
/// ## Transactional retirement
///
/// ```rust
/// let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
///     .await
///     .unwrap();
///
/// let delivery: Delivery<Value> = receiver.recv().await.unwrap();
///
/// // Transactionally retiring
/// let mut txn = Transaction::declare(&mut session, "controller-1", None)
///     .await
///     .unwrap();
/// txn.accept(&mut receiver, &delivery).await.unwrap();
/// txn.commit().await.unwrap();
/// ```
///
/// ## Transactional acquisition
///
/// ```rust
/// let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
///     .await
///     .unwrap();
///
/// // Transactionally retiring
/// let txn = Transaction::declare(&mut session, "controller-1", None)
///     .await
///     .unwrap();
/// let mut txn_acq = txn.acquire(&mut receiver, 2).await.unwrap();
/// let delivery1: Delivery<Value> = txn_acq.recv().await.unwrap();
/// let delivery2: Delivery<Value> = txn_acq.recv().await.unwrap();
/// txn_acq.accept(&delivery1).await.unwrap();
/// txn_acq.accept(&delivery2).await.unwrap();
/// txn_acq.commit().await.unwrap();
/// ```
#[derive(Debug)]
pub struct Transaction<'t> {
    controller: &'t Controller,
    declared: Declared,
    is_discharged: bool,
}

impl<'t> Transaction<'t> {
    /// Get the transaction ID
    pub fn txn_id(&self) -> &TransactionId {
        &self.declared.txn_id
    }

    /// Declares a transaction with a default controller
    ///
    /// The user needs to supply a name for the underlying control link.
    pub async fn declare(
        controller: &'t Controller,
        global_id: impl Into<Option<TransactionId>>,
    ) -> Result<Transaction<'t>, DeclareError> {
        let declared = controller.declare_inner(global_id.into()).await?;
        Ok(Self {
            controller,
            declared,
            is_discharged: false,
        })
    }

    /// Rollback the transaction
    ///
    /// This will send a [`Discharge`] with the `fail` field set to true
    ///
    /// If the coordinator is unable to complete the discharge, the coordinator MUST convey the
    /// error to the controller as a transaction-error
    pub async fn rollback(&mut self) -> Result<(), DischargeError> {
        if !self.is_discharged {
            self.controller
                .discharge(self.declared.txn_id.clone(), true)
                .await?;
            self.is_discharged = true;
            Ok(())
        } else {
            Err(DischargeError::AlreadyDischarged)
        }
    }

    /// Commit the transaction
    ///
    /// This will send a [`Discharge`] with the `fail` field set to false.
    ///
    /// If the coordinator is unable to complete the discharge, the coordinator MUST convey the
    /// error to the controller as a transaction-error
    pub async fn commit(&mut self) -> Result<(), DischargeError> {
        if !self.is_discharged {
            self.controller
                .discharge(self.declared.txn_id.clone(), false)
                .await?;
            self.is_discharged = true;
            Ok(())
        } else {
            Err(DischargeError::AlreadyDischarged)
        }
    }

    /// Post a transactional work
    pub async fn post<T>(
        &mut self,
        sender: &mut Sender,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<(), link::SendError>
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
        let settlement = sender.inner.send_with_state(sendable, Some(state)).await?;

        // On receiving a non-settled delivery associated with a live transaction, the transactional
        // resource MUST inform the controller of the presumptive terminal outcome before it can
        // successfully discharge the transaction. That is, the resource MUST send a disposition
        // performative which covers the posted transfer with the state of the delivery being a
        // transactional-state with the correct transaction identified, and a terminal outcome. This
        // informs the controller of the outcome that will be in effect at the point that the
        // transaction is successfully discharged.
        match settlement {
            Settlement::Settled => Ok(()),
            Settlement::Unsettled {
                _delivery_tag,
                outcome,
            } => match outcome.await? {
                DeliveryState::Received(_)
                | DeliveryState::Accepted(_)
                | DeliveryState::Rejected(_)
                | DeliveryState::Released(_)
                | DeliveryState::Modified(_)
                | DeliveryState::Declared(_) => Err(link::SendError::not_allowed(
                    "Expecting a TransactionalState".to_string(),
                )),
                DeliveryState::TransactionalState(txn) => {
                    // Interleaving transfer and disposition of different transactions
                    // isn't implemented
                    if txn.txn_id != self.declared.txn_id {
                        return Err(link::SendError::mismatched_transaction_id(
                            &self.declared.txn_id,
                            &txn.txn_id,
                        ));
                    }

                    match txn.outcome {
                        Some(Outcome::Accepted(_)) => Ok(()),
                        Some(Outcome::Rejected(value)) => Err(link::SendError::Rejected(value)),
                        Some(Outcome::Released(value)) => Err(link::SendError::Released(value)),
                        Some(Outcome::Modified(value)) => Err(link::SendError::Modified(value)),
                        Some(Outcome::Declared(_)) | None => {
                            Err(link::SendError::expecting_outcome())
                        }
                    }
                }
            },
        }
    }

    /// Associate an outcome with a transaction
    ///
    /// The delivery itself need not be associated with the same transaction as the outcome, or
    /// indeed with any transaction at all. However, the delivery MUST NOT be associated with a
    /// different non-discharged transaction than the outcome. If this happens then the control link
    /// MUST be terminated with a transaction-rollback error.
    pub async fn retire<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
        outcome: Outcome,
    ) -> Result<(), link::Error> {
        let txn_state = TransactionalState {
            txn_id: self.declared.txn_id.clone(),
            outcome: Some(outcome),
        };
        let state = DeliveryState::TransactionalState(txn_state);
        recver
            .inner
            .dispose(
                delivery.delivery_id.clone(),
                delivery.delivery_tag.clone(),
                state,
            )
            .await
    }

    /// Associate an Accepted outcome with a transaction
    pub async fn accept<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
    ) -> Result<(), link::Error> {
        let outcome = Outcome::Accepted(Accepted {});
        self.retire(recver, delivery, outcome).await
    }

    /// Associate a Rejected outcome with a transaction
    pub async fn reject<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
        error: impl Into<Option<definitions::Error>>,
    ) -> Result<(), link::Error> {
        let outcome = Outcome::Rejected(Rejected {
            error: error.into(),
        });
        self.retire(recver, delivery, outcome).await
    }

    /// Associate a Released outcome with a transaction
    pub async fn release<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
    ) -> Result<(), link::Error> {
        let outcome = Outcome::Released(Released {});
        self.retire(recver, delivery, outcome).await
    }

    /// Associate a Modified outcome with a transaction
    pub async fn modify<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
        modified: Modified,
    ) -> Result<(), link::Error> {
        let outcome = Outcome::Modified(modified.into());
        self.retire(recver, delivery, outcome).await
    }

    /// Acquire a transactional work
    ///
    /// This will send
    pub async fn acquire<'r>(
        self,
        recver: &'r mut Receiver,
        credit: SequenceNo,
    ) -> Result<TxnAcquisition<'t, 'r>, link::Error> {
        {
            let mut writer = recver.inner.link.flow_state.lock.write().await;
            match &mut writer.properties {
                Some(fields) => {
                    let key = Symbol::from("txn-id");
                    if fields.contains_key(&key) {
                        return Err(link::Error::Local(definitions::Error::new(
                            AmqpError::NotImplemented,
                            "Link endpoint is already associated with a transaction".to_string(),
                            None,
                        )));
                    }
                    let value = to_value(&self.declared.txn_id)?;
                    fields.insert(key, value);
                }
                None => {
                    let mut fields = Fields::new();
                    let key = Symbol::from("txn-id");
                    let value = to_value(&self.declared.txn_id)?;
                    fields.insert(key, value);
                }
            }
        }

        recver
            .inner
            .link
            .send_flow(&mut recver.inner.outgoing, Some(credit), None, false)
            .await?;
        Ok(TxnAcquisition {
            txn: self,
            recver,
        })
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
            let message = Message::<Discharge>::builder().value(discharge).build();
            let mut payload = BytesMut::new();
            let mut serializer = Serializer::from((&mut payload).writer());
            if let Err(error) = Serializable(message).serialize(&mut serializer) {
                tracing::error!(?error);
                return;
            }
            // let payload = BytesMut::from(payload);
            let payload = payload.freeze();
            let payload_copy = payload.clone();

            let mut inner_guard = self.controller.inner.borrow_mut();

            match inner_guard.link.flow_state.try_consume(1) {
                Ok(_) => {
                    let input_handle = match inner_guard.link.input_handle.clone().ok_or(AmqpError::IllegalState) {
                        Ok(handle) => handle,
                        Err(error) => {
                            tracing::error!(?error);
                            return
                        },
                    };
                    let handle = match inner_guard.link
                        .output_handle
                        .clone() {
                            Some(handle) => handle.into(),
                            None => return,
                        };
                    // let tag = self.flow_state.state().delivery_count().await.to_be_bytes();
                    let tag = match inner_guard.link.flow_state.state().lock.try_read() {
                        Ok(inner) => inner.delivery_count.to_be_bytes(),
                        Err(error) => {
                            tracing::error!(?error);
                            return 
                        },
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
                    match inner_guard.incoming.try_recv() {
                        Ok(_) => {
                            // The only frames that are relayed is detach
                            return
                        },
                        Err(error) => match error {
                            TryRecvError::Empty => {},
                            TryRecvError::Disconnected => return,
                        },
                    }

                    // Send out Rollback
                    let frame = LinkFrame::Transfer {
                        input_handle,
                        performative: transfer,
                        payload,
                    };
                    if let Err(_) = inner_guard.outgoing.blocking_send(frame) {
                        // Channel is already closed
                        return
                    }

                    // TODO: Wait for accept or not?
                    // The transfer is sent unsettled and will be 
                    // inserted into
                    let (tx, rx) = oneshot::channel();
                    let unsettled = UnsettledMessage::new(payload_copy, tx);
                    {
                        let mut guard = match inner_guard.link.unsettled.try_write() {
                            Ok(guard) => guard,
                            Err(error) => {
                                tracing::error!(?error);
                                return
                            },
                        };
                        guard.insert(delivery_tag, unsettled);
                    }
                    match rx.blocking_recv() {
                        Ok(state) => {
                            match state {
                                DeliveryState::Accepted(_) => {},
                                _ => {
                                    tracing::error!(error = ?state);
                                    return
                                }
                            }
                        },
                        Err(error) => {
                            tracing::error!(?error);
                            return
                        },
                    };
                }
                Err(error) => {
                    tracing::error!(?error);
                    return;
                }
            }
        }
    }
}
