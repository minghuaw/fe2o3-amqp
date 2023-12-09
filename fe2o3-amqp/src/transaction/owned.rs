//! Implements OwnedTransaction

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{Fields, SequenceNo},
    messaging::{DeliveryState, Outcome, SerializableBody},
    primitives::Symbol,
    transaction::{Declared, TransactionId, TransactionalState},
};
use serde_amqp::Value;

use crate::{
    endpoint::ReceiverLink,
    link::{
        delivery::{DeliveryFut, DeliveryInfo},
        DispositionError, FlowError,
    },
    session::SessionHandle,
    Receiver, Sendable, Sender,
};

use super::{
    Controller, ControllerSendError, OwnedDeclareError, OwnedDischargeError, PostError,
    TransactionDischarge, TransactionExt, TransactionalRetirement, TxnAcquisition, TXN_ID_KEY,
};

/// An owned transaction that has exclusive access to its own control link.
///
/// # Examples
///
/// Please note that only transactional posting has been tested.
///
/// ## Transactional posting
///
/// ```rust,ignore
/// let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
///     .await
///     .unwrap();
///
/// // Commit
/// let mut txn = OwnedTransaction::declare(&mut session, "owned-controller", None).await.unwrap();
/// txn.post(&mut sender, "hello").await.unwrap();
/// txn.post(&mut sender, "world").await.unwrap();
/// txn.commit().await.unwrap();
///
/// // Rollback
/// let mut txn = OwnedTransaction::declare(&mut session, "owned-controller", None).await.unwrap();
/// txn.post(&mut sender, "foo").await.unwrap();
/// txn.rollback().await.unwrap();
/// ```
///
/// ## Transactional retirement
///
/// ```rust,ignore
/// let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
///     .await
///     .unwrap();
///
/// let delivery: Delivery<Value> = receiver.recv().await.unwrap();
///
/// // Transactionally retiring
/// let mut txn = OwnedTransaction::declare(&mut session, "owned-controller", None).await.unwrap();
/// txn.accept(&mut receiver, &delivery).await.unwrap();
/// txn.commit().await.unwrap();
/// ```
///
/// ## Transactional acquisition
///
/// Please note that this is not supported on the resource side yet.
///
/// ```rust,ignore
/// let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
///     .await
///     .unwrap();
///
/// // Transactionally retiring
/// let mut txn = OwnedTransaction::declare(&mut session, "owned-controller", None).await.unwrap();
/// let mut txn_acq = txn.acquire(&mut receiver, 2).await.unwrap();
/// let delivery1: Delivery<Value> = txn_acq.recv().await.unwrap();
/// let delivery2: Delivery<Value> = txn_acq.recv().await.unwrap();
/// txn_acq.accept(&delivery1).await.unwrap();
/// txn_acq.accept(&delivery2).await.unwrap();
/// txn_acq.commit().await.unwrap();
/// ```
#[derive(Debug)]
pub struct OwnedTransaction {
    controller: Controller,
    declared: Declared,
    is_discharged: bool,
}

#[async_trait]
impl TransactionDischarge for OwnedTransaction {
    type Error = OwnedDischargeError;

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

    async fn rollback(mut self) -> Result<(), Self::Error> {
        self.discharge(true).await?;
        self.controller.close().await?;
        Ok(())
    }

    async fn commit(mut self) -> Result<(), Self::Error> {
        self.discharge(false).await?;
        self.controller.close().await?;
        Ok(())
    }
}

#[async_trait]
impl TransactionalRetirement for OwnedTransaction {
    type RetireError = DispositionError;

    /// Associate an outcome with a transaction
    ///
    /// The delivery itself need not be associated with the same transaction as the outcome, or
    /// indeed with any transaction at all. However, the delivery MUST NOT be associated with a
    /// different non-discharged transaction than the outcome. If this happens then the control link
    /// MUST be terminated with a transaction-rollback error.
    async fn retire<T>(
        &self,
        recver: &mut Receiver,
        delivery: T,
        outcome: Outcome,
    ) -> Result<(), Self::RetireError>
    where
        T: Into<DeliveryInfo> + Send,
    {
        let txn_state = TransactionalState {
            txn_id: self.declared.txn_id.clone(),
            outcome: Some(outcome),
        };
        let state = DeliveryState::TransactionalState(txn_state);
        recver.inner.dispose(delivery, None, state).await
    }
}

impl TransactionExt for OwnedTransaction {
    fn txn_id(&self) -> &TransactionId {
        &self.declared.txn_id
    }
}

impl OwnedTransaction {
    /// Declare an transaction with an owned control link
    pub async fn declare<R>(
        session: &mut SessionHandle<R>,
        name: impl Into<String>,
        global_id: impl Into<Option<TransactionId>>,
    ) -> Result<OwnedTransaction, OwnedDeclareError> {
        let controller = Controller::attach(session, name).await?;
        Self::declare_with_controller(controller, global_id)
            .await
            .map_err(Into::into)
    }

    /// Declare an transaction with an owned control link
    pub async fn declare_with_controller(
        controller: Controller,
        global_id: impl Into<Option<TransactionId>>,
    ) -> Result<OwnedTransaction, ControllerSendError> {
        let declared = controller.declare_inner(global_id.into()).await?;
        Ok(Self {
            controller,
            declared,
            is_discharged: false,
        })
    }

    /// Post a ref of transactional work and wait for the acknowledgement.
    pub async fn post_batchable_ref<T: SerializableBody>(
        &self,
        sender: &mut Sender,
        sendable: &Sendable<T>,
    ) -> Result<DeliveryFut<Result<Outcome, PostError>>, PostError> {
        let state = TransactionalState {
            txn_id: self.declared.txn_id.clone(),
            outcome: None,
        };
        let state = DeliveryState::TransactionalState(state);
        let settlement = sender
            .inner
            .send_ref_with_state::<T, PostError>(sendable, Some(state), false)
            .await?;

        Ok(DeliveryFut::from(settlement))
    }

    /// Post a ref of transactional work
    pub async fn post_ref<T: SerializableBody>(
        &self,
        sender: &mut Sender,
        sendable: &Sendable<T>,
    ) -> Result<Outcome, PostError> {
        let fut = self.post_batchable_ref(sender, sendable).await?;
        fut.await
    }

    /// Post a transactional work without waiting for the acknowledgement.
    pub async fn post_batchable<T>(
        &self,
        sender: &mut Sender,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<DeliveryFut<Result<Outcome, PostError>>, PostError>
    where
        T: SerializableBody,
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
            .send_with_state::<T, PostError>(sendable, Some(state), true)
            .await?;

        Ok(DeliveryFut::from(settlement))
    }

    /// Post a transactional work
    pub async fn post<T>(
        &self,
        sender: &mut Sender,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<Outcome, PostError>
    where
        T: SerializableBody,
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
            .send_with_state::<T, PostError>(sendable, Some(state), false)
            .await?;

        let fut = DeliveryFut::from(settlement);

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
    pub async fn acquire(
        self,
        recver: &mut Receiver,
        credit: SequenceNo,
    ) -> Result<TxnAcquisition<'_, OwnedTransaction>, FlowError> {
        {
            let mut writer = recver.inner.link.flow_state.lock.write();
            let value = Value::Binary(self.declared.txn_id.clone());
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
