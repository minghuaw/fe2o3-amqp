//! Transaction

use crate::{
    endpoint::{Settlement, ReceiverLink},
    link::{self},
    Receiver, Sendable, Sender, Delivery, session::SessionHandle,
};
use fe2o3_amqp_types::{
    messaging::{DeliveryState, Outcome, Accepted, Modified, Rejected, Released},
    transaction::{Declared, TransactionalState, Coordinator, TransactionId}, definitions::{self, AmqpError, Fields, SequenceNo}, primitives::Symbol,
};

mod controller;
pub use controller::*;

mod error;
pub use error::*;
use serde_amqp::to_value;

/// A transaction scope
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
    /// Daclares a transaction
    /// 
    /// The user needs to supply a name for the underlying control link.
    pub async fn declare<R>(session: &mut SessionHandle<R>, name: impl Into<String>, global_id: Option<TransactionId>) -> Result<Self, DeclareError> {
        let controller = Controller::attach(session, name, Coordinator::default()).await?
            .declare(global_id).await?;
        let txn = Self { controller };
        Ok(txn)
    }

    /// Rollback the transaction
    pub async fn rollback(mut self) -> Result<(), link::Error> {
        self.controller.rollback().await?;
        self.controller.close().await?;
        Ok(())
    }

    /// Commit the transaction
    pub async fn commit(mut self) -> Result<(), link::Error> {
        self.controller.commit().await?;
        self.controller.close().await?;
        Ok(())
    }

    /// Post a transactional work
    ///
    /// Performing multiple works for different transactions on a single sender link
    /// is not implemented yet.
    pub async fn post<T>(
        &mut self,
        sender: &mut Sender,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<(), link::Error>
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
            txn_id: self.controller.transaction_id().clone(),
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
                | DeliveryState::Declared(_) => Err(link::Error::not_allowed(
                    "Expecting a TransactionalState".to_string(),
                )),
                DeliveryState::TransactionalState(txn) => {
                    // TODO: What if there are two separate transactions?
                    if txn.txn_id != *self.controller.transaction_id() {
                        return Err(link::Error::mismatched_transaction_id(
                            self.controller.transaction_id(),
                            &txn.txn_id,
                        ));
                    }

                    match txn.outcome {
                        Some(Outcome::Accepted(_)) => Ok(()),
                        Some(Outcome::Rejected(value)) => Err(link::Error::Rejected(value)),
                        Some(Outcome::Released(value)) => Err(link::Error::Released(value)),
                        Some(Outcome::Modified(value)) => Err(link::Error::Modified(value)),
                        Some(Outcome::Declared(_)) | None => Err(link::Error::expecting_outcome()),
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
    pub async fn retire<T>(&mut self, recver: &mut Receiver, delivery: &Delivery<T>, outcome: Outcome) -> Result<(), link::Error> {
        let txn_state = TransactionalState {
            txn_id: self.controller.transaction_id().clone(),
            outcome: Some(outcome),
        };
        let state = DeliveryState::TransactionalState(txn_state);
        recver.dispose(delivery.delivery_id.clone(), delivery.delivery_tag.clone(), state).await
    }

    /// Associate an Accepted outcome with a transaction
    pub async fn accept<T>(&mut self, recver: &mut Receiver, delivery: &Delivery<T>) -> Result<(), link::Error> {
        let outcome = Outcome::Accepted(Accepted {} );
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
    pub async fn release<T>(&mut self, recver: &mut Receiver, delivery: &Delivery<T>) -> Result<(), link::Error> {
        let outcome = Outcome::Released(Released {});
        self.retire(recver, delivery, outcome).await
    }
    
    /// Associate a Modified outcome with a transaction
    pub async fn modify<T>(
        &mut self,
        recver: &mut Receiver,
        delivery: &Delivery<T>,
        modified: impl Into<Modified>,
    ) -> Result<(), link::Error> {
        let outcome = Outcome::Modified(modified.into());
        self.retire(recver, delivery, outcome).await
    }

    /// Acquire a transactional work
    /// 
    /// This will send 
    pub async fn acquire<'t, 'r>(&'t mut self, recver: &'r mut Receiver, credit: SequenceNo) -> Result<TxnAcquisition<'t, 'r>, link::Error> {
        {
            let mut writer = recver.link.flow_state.lock.write().await;
            match &mut writer.properties {
                Some(fields) => {
                    let key = Symbol::from("txn-id");
                    if fields.contains_key(&key) {
                        return Err(link::Error::Local(definitions::Error::new(
                            AmqpError::NotImplemented,
                            "Link endpoint is already associated with a transaction".to_string(),
                            None
                        )))
                    }
                    let value = to_value(self.controller.transaction_id())?;
                    fields.insert(key, value);
                }
                None => {
                    let mut fields = Fields::new();
                    let key = Symbol::from("txn-id");
                    let value = to_value(self.controller.transaction_id())?;
                    fields.insert(key, value);
                },
            }
        }

        recver.link.send_flow(&mut recver.outgoing, Some(credit), None, false).await?;
        Ok(TxnAcquisition {
            txn: self,
            recver,
            cleaned_up: false,
        })
    }

}

/// 4.4.3 Transactional Acquisition
#[derive(Debug)]
pub struct TxnAcquisition<'t, 'r> {
    /// The transaction context of this acquisition
    pub txn: &'t mut Transaction,
    /// The receiver that is associated with the acquisition
    pub recver: &'r mut Receiver,
    cleaned_up: bool,
}

impl<'t, 'r> TxnAcquisition<'t, 'r> {
    /// Clear txn-id from link and set link to drain
    pub async fn cleanup(&mut self, recver: &mut Receiver) -> Result<(), link::Error> {
        // clear txn-id 
        {
            let mut writer = recver.link.flow_state.lock.write().await;
            let key = Symbol::from("txn-id");
            writer.properties.as_mut()
                .map(|map| map.remove(&key));
        }

        // set drain to true
        recver.link.send_flow(&mut recver.outgoing, Some(0), Some(true), true).await?;
        
        self.cleaned_up = true;
        Ok(())
    }
}

impl<'t, 'r> Drop for TxnAcquisition<'t, 'r> {
    fn drop(&mut self) {
        if !self.cleaned_up {
            // clear txn-id from the link's properties
            {
                let mut writer = self.recver.link.flow_state.lock.blocking_write();
                let key = Symbol::from("txn-id");
                writer.properties.as_mut()
                    .map(|fields| fields.remove(&key));
            }
    
            // Set drain to true
            if let Some(sender) = self.recver.outgoing.get_ref() {
                if let Err(err) = (&mut self.recver.link).blocking_send_flow(sender, Some(0), Some(true), true) {
                    tracing::error!("error {:?}", err)
                }
            }
        }
    }
}