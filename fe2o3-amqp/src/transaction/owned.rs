//! Implements OwnedTransaction


use fe2o3_amqp_types::transaction::{Declared, TransactionId};

use crate::{link::DispositionError, session::SessionHandle};

use super::{
    Controller, ControllerSendError, OwnedDeclareError, OwnedDischargeError, TransactionAcquisition,
    TransactionBase, TransactionDischarge, TransactionExt, TransactionPosting,
    TransactionRetirement,
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
/// use fe2o3_amqp::transaction::{
///     OwnedTransaction, TransactionDischarge, TransactionPosting,
/// };
///
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
/// use fe2o3_amqp::transaction::{
///     OwnedTransaction, TransactionDischarge, TransactionRetirement,
/// };
///
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
/// use fe2o3_amqp::transaction::{
///     OwnedTransaction, TransactionAcquisition, TransactionRetirement,
/// };
///
/// let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
///     .await
///     .unwrap();
///
/// // Transactionally acquiring
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


impl TransactionRetirement for OwnedTransaction {
    type RetireError = DispositionError;
}

impl TransactionBase for OwnedTransaction {
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
}

impl TransactionPosting for OwnedTransaction {}

impl TransactionAcquisition for OwnedTransaction {}

impl TransactionExt for OwnedTransaction {}
