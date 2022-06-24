//! 4.4.3 Transactional Acquisition

use fe2o3_amqp_types::{
    definitions::{self, SequenceNo},
    messaging::Modified,
    primitives::Symbol,
    transaction::TransactionId,
};

use crate::{
    endpoint::ReceiverLink,
    link::{self, delivery},
    Delivery, Receiver,
};

use super::{Transaction, DischargeError};

/// 4.4.3 Transactional Acquisition
/// 
/// # Lifetime parameters
/// 
/// 't: lifetime of the Transaction
/// 'r: lifetime of the Receiver
#[derive(Debug)]
pub struct TxnAcquisition<'t, 'r> {
    /// The transaction context of this acquisition
    pub(super) txn: Transaction<'t>,
    /// The receiver that is associated with the acquisition
    pub(super) recver: &'r mut Receiver,
    // pub(super) cleaned_up: bool,
}

impl<'t, 'r> TxnAcquisition<'t, 'r> {
    /// Get an immutable reference to the underlying transaction
    pub fn txn(&self) -> &Transaction {
        &self.txn
    }

    /// Get a mutable reference to the underlying transaction
    pub fn txn_mut(&'t mut self) -> &'t mut Transaction {
        &mut self.txn
    }

    /// Get the transaction ID
    pub fn txn_id(&self) -> &TransactionId {
        self.txn.txn_id()
    }

    /// Clear transaction-id from link and set link to drain
    pub async fn cleanup(&mut self) -> Result<(), link::Error> {
        // clear txn-id
        {
            let mut writer = self.recver.inner.link.flow_state.lock.write().await;
            let key = Symbol::from("txn-id");
            writer.properties.as_mut().map(|map| map.remove(&key));
        }

        // set drain to true
        self.recver
            .inner
            .link
            .send_flow(&mut self.recver.inner.outgoing, Some(0), Some(true), true)
            .await?;

        // self.cleaned_up = true;
        Ok(())
    }

    /// Transactionally acquire a message
    pub async fn recv<T>(&mut self) -> Result<delivery::Delivery<T>, link::Error>
    where
        T: for<'de> serde::Deserialize<'de> + Send,
    {
        self.recver.recv().await
    }

    /// Set the credit
    pub async fn set_credit(&mut self, credit: SequenceNo) -> Result<(), link::Error> {
        // "txn-id" should be already included in the link's properties map
        self.recver.set_credit(credit).await
    }

    /// Commit the transactional acquisition
    pub async fn commit(mut self) -> Result<(), DischargeError> {
        self.cleanup().await?;
        self.txn.commit_inner().await?;
        Ok(())
    }

    /// Rollback the transactional acquisition
    pub async fn rollback(mut self) -> Result<(), DischargeError> {
        self.cleanup().await?;
        self.txn.rollback_inner().await?;
        Ok(())
    }

    /// Accept the message
    pub async fn accept<T>(&mut self, delivery: &Delivery<T>) -> Result<(), link::Error> {
        self.txn.accept(self.recver, delivery).await
    }

    /// Reject the message
    pub async fn reject<T>(
        &mut self,
        delivery: &Delivery<T>,
        error: impl Into<Option<definitions::Error>>,
    ) -> Result<(), link::Error> {
        self.txn.reject(self.recver, delivery, error).await
    }

    /// Release the message
    pub async fn release<T>(&mut self, delivery: &Delivery<T>) -> Result<(), link::Error> {
        self.txn.release(self.recver, delivery).await
    }

    /// Modify the message
    pub async fn modify<T>(
        &mut self,
        delivery: &Delivery<T>,
        modified: Modified,
    ) -> Result<(), link::Error> {
        self.txn.modify(self.recver, delivery, modified).await
    }
}

impl<'t, 'r> Drop for TxnAcquisition<'t, 'r> {
    fn drop(&mut self) {
        if !self.txn.is_discharged {
            // clear txn-id from the link's properties
            {
                let mut writer = self.recver.inner.link.flow_state.lock.blocking_write();
                let key = Symbol::from("txn-id");
                writer.properties.as_mut().map(|fields| fields.remove(&key));
            }

            // Set drain to true
            if let Err(err) = (&mut self.recver.inner.link).blocking_send_flow(
                &self.recver.inner.outgoing,
                Some(0),
                Some(true),
                true,
            ) {
                tracing::error!("error {:?}", err)
            }
        }
    }
}
