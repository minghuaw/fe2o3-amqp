//! Transaction

use crate::{
    endpoint::{ReceiverLink, Settlement},
    link::{self},
    session::SessionHandle,
    Delivery, Receiver, Sendable, Sender,
};
use fe2o3_amqp_types::{
    definitions::{self, AmqpError, Fields, SequenceNo},
    messaging::{Accepted, DeliveryState, Modified, Outcome, Rejected, Released},
    primitives::Symbol,
    transaction::{Coordinator, Declared, TransactionId, TransactionalState, TxnCapability},
};

mod controller;
pub use controller::*;

mod error;
pub use error::*;
use serde_amqp::to_value;

mod acquisition;
pub use acquisition::*;

mod control_link_frame;

pub mod coordinator;
pub mod frame;
pub mod manager;

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
pub struct Transaction {
    controller: Controller<Declared>,
}

impl From<Controller<Declared>> for Transaction {
    fn from(controller: Controller<Declared>) -> Self {
        Self { controller }
    }
}

impl Transaction {
    /// Get the transaction ID
    pub fn txn_id(&self) -> &TransactionId {
        self.controller.txn_id()
    }

    /// Declares a transaction with a default controller
    ///
    /// The user needs to supply a name for the underlying control link.
    pub async fn declare<R>(
        session: &mut SessionHandle<R>,
        name: impl Into<String>,
        global_id: impl Into<Option<TransactionId>>,
    ) -> Result<Self, DeclareError> {
        let controller = Controller::attach(session, name, Coordinator::default())
            .await?
            .declare(global_id)
            .await?;
        Ok(Self { controller })
    }

    /// Declares a transaction with a default controller and a list of desired transaction capabilities
    pub async fn declare_with_desired_capabilities<R>(
        session: &mut SessionHandle<R>,
        name: impl Into<String>,
        capabiltiies: impl IntoIterator<Item = TxnCapability>,
        global_id: Option<TransactionId>,
    ) -> Result<Self, DeclareError> {
        let coordinator = Coordinator {
            capabilities: Some(capabiltiies.into_iter().collect()),
        };
        let controller = Controller::builder()
            .name(name.into())
            .coordinator(coordinator)
            .attach(session)
            .await?
            .declare(global_id)
            .await?;
        Ok(Self { controller })
    }

    /// Declares a transaction with an undeclared controller
    pub async fn declare_with_controller<R>(
        controller: Controller<Undeclared>,
        global_id: Option<TransactionId>,
    ) -> Result<Self, DeclareError> {
        let controller = controller.declare(global_id).await?;
        Ok(Self { controller })
    }

    /// Rollback the transaction
    pub async fn rollback(mut self) -> Result<(), link::SendError> {
        self.controller.rollback().await?;
        self.controller.close().await?;
        Ok(())
    }

    /// Commit the transaction
    pub async fn commit(mut self) -> Result<(), link::SendError> {
        self.controller.commit().await?;
        self.controller.close().await?;
        Ok(())
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
            txn_id: self.controller.txn_id().clone(),
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
                    if txn.txn_id != *self.controller.txn_id() {
                        return Err(link::SendError::mismatched_transaction_id(
                            self.controller.txn_id(),
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
            txn_id: self.controller.txn_id().clone(),
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
    ) -> Result<TxnAcquisition<'r>, link::Error> {
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
                    let value = to_value(self.controller.txn_id())?;
                    fields.insert(key, value);
                }
                None => {
                    let mut fields = Fields::new();
                    let key = Symbol::from("txn-id");
                    let value = to_value(self.controller.txn_id())?;
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
            cleaned_up: false,
        })
    }
}
