//! Transaction

use crate::{Sender, Receiver, link::sender::SenderInner};
use fe2o3_amqp_types::transaction::Declared;

mod controller;
pub use controller::*;

mod error;
pub use error::*;

/// A transaction scope
#[derive(Debug)]
pub struct Transaction {
    controller: Controller<Declared>
}

impl Transaction {
    /// Daclares a transaction
    pub async fn declare() -> Result<Self, ()> {
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
    pub async fn post(&mut self, sender: &mut Sender) -> Result<(), ()> {
        todo!()
    }

    /// Acquire a transactional work
    pub async fn acquire<T>(&mut self, recver: &mut Receiver) -> Result<T, ()> {
        todo!()
    }

    // pub async fn retire<T>(&mut self, endpoint)
}