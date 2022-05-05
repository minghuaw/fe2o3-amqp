//! Transaction

use crate::{link::{sender::SenderInner, self}, Receiver, Sender, Sendable, endpoint::Settlement};
use fe2o3_amqp_types::{transaction::{Declared, TransactionalState}, messaging::{DeliveryState, Outcome}};

mod controller;
pub use controller::*;

mod error;
pub use error::*;

/// A transaction scope
#[derive(Debug)]
pub struct Transaction {
    controller: Controller<Declared>,
}

impl Transaction {
    /// Daclares a transaction
    pub async fn declare() -> Result<Self, ()> {
        todo!()
    }

    /// Declare a transaction with a customized controller
    pub async fn declare_with_controller(controller: Controller<Undeclared>) -> Result<Self, ()> {
        todo!()
    }

    /// Rollback the transaction
    pub async fn rollback(self) -> Result<(), ()> {
        todo!()
    }

    /// Commit the transaction
    pub async fn commit(self) -> Result<(), ()> {
        todo!()
    }

    /// Post a transactional work
    /// 
    /// Performing multiple works for different transactions on a single sender link 
    /// is not implemented yet.
    pub async fn post<T>(&mut self, sender: &mut Sender, sendable: impl Into<Sendable<T>>) -> Result<(), link::Error> 
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
                outcome 
            } => match outcome.await? {
                DeliveryState::Received(_) |
                DeliveryState::Accepted(_) |
                DeliveryState::Rejected(_) |
                DeliveryState::Released(_) |
                DeliveryState::Modified(_) |
                DeliveryState::Declared(_) => Err(link::Error::not_allowed("Expecting a TransactionalState".to_string())),
                DeliveryState::TransactionalState(txn) => {
                    // TODO: What if there are two separate transactions?
                    if txn.txn_id != *self.controller.transaction_id() {
                        return Err(link::Error::mismatched_transaction_id(self.controller.transaction_id(), &txn.txn_id))
                    }

                    match txn.outcome {
                        Some(Outcome::Accepted(_)) => Ok(()),
                        Some(Outcome::Rejected(value)) => Err(link::Error::Rejected(value)),
                        Some(Outcome::Released(value)) => Err(link::Error::Released(value)),
                        Some(Outcome::Modified(value)) => Err(link::Error::Modified(value)),
                        Some(Outcome::Declared(_)) 
                        | None => Err(link::Error::expecting_outcome()),
                    }
                },
            }
        }
    }

    /// Acquire a transactional work
    pub async fn acquire<T>(&mut self, recver: &mut Receiver) -> Result<T, ()> {
        todo!()
    }

    // pub async fn retire<T>(&mut self, endpoint)
}
