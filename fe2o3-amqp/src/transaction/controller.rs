use fe2o3_amqp_types::{
    definitions::{self, SenderSettleMode},
    messaging::{Accepted, DeliveryState, Message},
    transaction::{Coordinator, Declare, Declared, Discharge, TransactionId},
};
use tokio::sync::{oneshot, Mutex};

use crate::{
    endpoint::Settlement,
    link::{
        self,
        builder::{WithSource, WithoutName, WithoutTarget},
        role,
        sender::SenderInner,
        shared_inner::LinkEndpointInnerDetach,
        LinkStateError, SendError, SenderAttachError, SenderLink,
    },
    session::SessionHandle,
    Sendable,
};

use super::ControllerSendError;
#[cfg(docsrs)]
use super::{OwnedTransaction, Transaction};

pub(crate) type ControlLink = SenderLink<Coordinator>;

/// Transaction controller
///
/// This represents the controller side of a control link. The usage is similar to that of [`crate::Sender`]
/// but doesn't allow user to send any custom messages as the control link is purely used for declaring
/// and discharging transactions. Please also see [`Transaction`] and [`OwnedTransaction`]
///
/// # Example
///
/// ```rust
/// let controller = Controller::attach(&mut session, "controller").await.unwrap();
/// let mut txn = Transaction::declare(&controller, None).await.unwrap();
/// txn.commit().await.unwrap();
/// controller.close().await.unwrap();
/// ```
#[derive(Debug)]
pub struct Controller {
    pub(crate) inner: Mutex<SenderInner<ControlLink>>,
}

#[inline]
async fn send_on_control_link<T>(
    sender: &mut SenderInner<ControlLink>,
    sendable: Sendable<T>,
) -> Result<oneshot::Receiver<Option<DeliveryState>>, link::SendError>
where
    T: serde::Serialize,
{
    match sender.send(sendable).await? {
        Settlement::Settled(_) => Err(SendError::IllegalDeliveryState),
        Settlement::Unsettled {
            delivery_tag: _,
            outcome,
        } => Ok(outcome),
    }
}

impl Controller {
    /// Creates a new builder for controller
    pub fn builder(
    ) -> link::builder::Builder<role::Sender, Coordinator, WithoutName, WithSource, WithoutTarget>
    {
        link::builder::Builder::<role::Sender, Coordinator, WithoutName, WithSource, WithoutTarget>::new()
    }

    /// Close the control link with error
    pub async fn close_with_error(
        mut self,
        error: definitions::Error,
    ) -> Result<(), link::DetachError> {
        self.inner.get_mut().close_with_error(Some(error)).await
    }

    /// Close the link
    pub async fn close(mut self) -> Result<(), link::DetachError> {
        self.inner.get_mut().close_with_error(None).await
    }

    /// Attach the controller with the default [`Coordinator`]
    pub async fn attach<R>(
        session: &mut SessionHandle<R>,
        name: impl Into<String>,
    ) -> Result<Self, SenderAttachError> {
        Self::attach_with_coordinator(session, name, Coordinator::default()).await
    }

    /// Attach the controller with a customized [`Coordinator`]
    pub async fn attach_with_coordinator<R>(
        session: &mut SessionHandle<R>,
        name: impl Into<String>,
        coordinator: Coordinator,
    ) -> Result<Self, SenderAttachError> {
        Self::builder()
            .name(name)
            .coordinator(coordinator)
            .sender_settle_mode(SenderSettleMode::Unsettled)
            .attach(session)
            .await
    }

    pub(crate) async fn declare_inner(
        &self,
        global_id: Option<TransactionId>,
    ) -> Result<Declared, ControllerSendError> {
        // To begin transactional work, the transaction controller needs to obtain a transaction
        // identifier from the resource. It does this by sending a message to the coordinator whose
        // body consists of the declare type in a single amqp-value section. Other standard message
        // sections such as the header section SHOULD be ignored.
        let declare = Declare { global_id };
        let message = Message::builder().value(declare).build();
        // This message MUST NOT be sent settled as the sender is REQUIRED to receive and interpret
        // the outcome of the declare from the receiver
        let sendable = Sendable::builder().message(message).settled(false).build();

        send_on_control_link(&mut *self.inner.lock().await, sendable)
            .await?
            .await
            .map_err(|_| LinkStateError::IllegalSessionState)?
            .ok_or(ControllerSendError::NonTerminalDeliveryState)?
            .declared_or_else(|state| {
                if let DeliveryState::Rejected(rejected) = state {
                    ControllerSendError::Rejected(rejected)
                } else {
                    ControllerSendError::IllegalDeliveryState
                }
            })
    }

    /// Discharge
    pub(crate) async fn discharge(
        &self,
        txn_id: TransactionId,
        fail: impl Into<Option<bool>>,
    ) -> Result<Accepted, ControllerSendError> {
        let discharge = Discharge {
            txn_id,
            fail: fail.into(),
        };
        // As with the declare message, it is an error if the sender sends the transfer pre-settled.
        let message = Message::builder().value(discharge).build();
        let sendable = Sendable::builder().message(message).settled(false).build();

        send_on_control_link(&mut *self.inner.lock().await, sendable)
            .await?
            .await
            .map_err(|_| LinkStateError::IllegalSessionState)?
            .ok_or(ControllerSendError::NonTerminalDeliveryState)?
            .accepted_or_else(|state| {
                if let DeliveryState::Rejected(rejected) = state {
                    ControllerSendError::Rejected(rejected)
                } else {
                    ControllerSendError::IllegalDeliveryState
                }
            })
    }
}

// TODO: implement Drop for controller to drop all non-committed transactions
