//! 4.4.3 Transactional Acquisition

use fe2o3_amqp_types::{
    definitions::{self, SequenceNo},
    messaging::{FromBody, Modified},
    transaction::TransactionId,
};

use crate::{
    endpoint::ReceiverLink,
    link::{delivery, DispositionError, FlowError, RecvError, SendError},
    Delivery, Receiver,
};

use super::{TransactionDischarge, TransactionExt, TransactionalRetirement, TXN_ID_KEY};

/// 4.4.3 Transactional Acquisition
///
/// # Lifetime parameters
///
/// 't: lifetime of the Transaction
/// 'r: lifetime of the Receiver
#[derive(Debug)]
pub struct TxnAcquisition<'r, Txn>
where
    Txn: TransactionExt,
{
    /// The transaction context of this acquisition
    pub(super) txn: Txn,
    /// The receiver that is associated with the acquisition
    pub(super) recver: &'r mut Receiver,
    // pub(super) cleaned_up: bool,
}

impl<'r, Txn> TxnAcquisition<'r, Txn>
where
    Txn: TransactionExt
        + TransactionDischarge<Error = SendError>
        + TransactionalRetirement<RetireError = DispositionError>
        + Send
        + Sync,
{
    /// Get an immutable reference to the underlying transaction
    pub fn txn(&self) -> &Txn {
        &self.txn
    }

    /// Get a mutable reference to the underlying transaction
    pub fn txn_mut(&mut self) -> &mut Txn {
        &mut self.txn
    }

    /// Get the transaction ID
    pub fn txn_id(&self) -> &TransactionId {
        self.txn.txn_id()
    }

    /// Clear transaction-id from link and set link to drain
    pub async fn cleanup(&mut self) -> Result<(), FlowError> {
        // clear txn-id
        {
            let mut writer = self.recver.inner.link.flow_state.lock.write();
            writer.properties.as_mut().map(|map| map.swap_remove(TXN_ID_KEY));
        }

        // set drain to true
        self.recver
            .inner
            .link
            .send_flow(&self.recver.inner.outgoing, Some(0), Some(true), true, false)
            .await?;

        // self.cleaned_up = true;
        Ok(())
    }

    /// Transactionally acquire a message
    pub async fn recv<T>(&mut self) -> Result<delivery::Delivery<T>, RecvError>
    where
        for<'de> T: FromBody<'de> + Send,
    {
        self.recver.recv().await
    }

    /// Set the credit
    pub async fn set_credit(&mut self, credit: SequenceNo) -> Result<(), FlowError> {
        // "txn-id" should be already included in the link's properties map
        self.recver.set_credit(credit).await
    }

    /// Commit the transactional acquisition
    pub async fn commit(mut self) -> Result<(), SendError> {
        self.cleanup().await?;
        self.txn.discharge(false).await?;
        Ok(())
    }

    /// Rollback the transactional acquisition
    pub async fn rollback(mut self) -> Result<(), SendError> {
        self.cleanup().await?;
        self.txn.discharge(true).await?;
        Ok(())
    }

    /// Accept the message
    pub async fn accept<T>(&mut self, delivery: &Delivery<T>) -> Result<(), DispositionError>
    where
        T: Send + Sync,
    {
        self.txn.accept(self.recver, delivery).await
    }

    /// Reject the message
    pub async fn reject<T>(
        &mut self,
        delivery: &Delivery<T>,
        error: impl Into<Option<definitions::Error>>,
    ) -> Result<(), DispositionError>
    where
        T: Send + Sync,
    {
        self.txn.reject(self.recver, delivery, error.into()).await
    }

    /// Release the message
    pub async fn release<T>(&mut self, delivery: &Delivery<T>) -> Result<(), DispositionError>
    where
        T: Send + Sync,
    {
        self.txn.release(self.recver, delivery).await
    }

    /// Modify the message
    pub async fn modify<T>(
        &mut self,
        delivery: &Delivery<T>,
        modified: Modified,
    ) -> Result<(), DispositionError>
    where
        T: Send + Sync,
    {
        self.txn.modify(self.recver, delivery, modified).await
    }
}

impl<'r, T> Drop for TxnAcquisition<'r, T>
where
    T: TransactionExt,
{
    fn drop(&mut self) {
        if !self.txn.is_discharged() {
            // clear txn-id from the link's properties
            {
                let mut writer = self.recver.inner.link.flow_state.lock.write();
                writer
                    .properties
                    .as_mut()
                    .map(|fields| fields.swap_remove(TXN_ID_KEY));
            }

            // Set drain to true
            if let Err(_err) = self.recver.inner.link.blocking_send_flow(
                &self.recver.inner.outgoing,
                Some(0),
                Some(true),
                true,
                false
            ) {
                #[cfg(feature = "tracing")]
                tracing::error!("error {:?}", _err);
                #[cfg(feature = "log")]
                log::error!("error {:?}", _err);
            }
        }
    }
}
