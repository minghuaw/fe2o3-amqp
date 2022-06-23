use std::cell::RefCell;

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, SenderSettleMode},
    messaging::{DeliveryState, Message},
    transaction::{Coordinator, Declare, Declared, Discharge, TransactionId},
};
use tokio::sync::oneshot;

use crate::{
    endpoint::Settlement,
    link::{
        self,
        builder::{WithoutName, WithoutTarget},
        delivery::UnsettledMessage,
        role,
        sender::SenderInner,
        AttachError, Link, SenderFlowState,
    },
    session::SessionHandle,
    Sendable,
};

pub(crate) type ControlLink = Link<role::Sender, Coordinator, SenderFlowState, UnsettledMessage>;

/// Transaction controller
///
/// # Type parameter `S`
#[derive(Debug)]
pub struct Controller {
    pub(crate) inner: RefCell<SenderInner<ControlLink>>,
}

#[inline]
async fn send_on_control_link<T>(
    sender: &mut SenderInner<ControlLink>,
    sendable: Sendable<T>,
) -> Result<oneshot::Receiver<DeliveryState>, link::SendError>
where
    T: serde::Serialize,
{
    match sender.send(sendable).await? {
        Settlement::Settled => {
            let err = link::SendError::Local(definitions::Error::new(
                AmqpError::InternalError,
                "Declare cannot be sent settled".to_string(),
                None,
            ));
            return Err(err);
        }
        Settlement::Unsettled {
            _delivery_tag,
            outcome,
        } => Ok(outcome),
    }
}

impl Controller {
    /// Close the control link with error
    pub async fn close_with_error(
        &mut self,
        error: definitions::Error,
    ) -> Result<(), link::DetachError> {
        self.inner.get_mut().close_with_error(Some(error)).await
    }

    /// Close the link
    pub async fn close(&mut self) -> Result<(), link::DetachError> {
        self.inner.get_mut().close_with_error(None).await
    }
}

impl Controller {
    /// Creates a new builder for controller
    pub fn builder() -> link::builder::Builder<role::Sender, Coordinator, WithoutName, WithoutTarget>
    {
        link::builder::Builder::<role::Sender, Coordinator, WithoutName, WithoutTarget>::new()
    }

    /// Attach the controller with the default [`Coordinator`]
    pub async fn attach<R>(
        session: &mut SessionHandle<R>,
        name: impl Into<String>,
    ) -> Result<Self, AttachError> {
        Self::attach_with_coordinator(session, name, Coordinator::default()).await
    }

    /// Attach the controller with a customized [`Coordinator`]
    pub async fn attach_with_coordinator<R>(
        session: &mut SessionHandle<R>,
        name: impl Into<String>,
        coordinator: Coordinator,
    ) -> Result<Self, AttachError> {
        Self::builder()
            .name(name)
            .coordinator(coordinator)
            .sender_settle_mode(SenderSettleMode::Unsettled)
            .attach(session)
            .await
    }

    // /// Declare a transaction
    // pub async fn declare<'a>(
    //     &'a mut self,
    //     global_id: impl Into<Option<TransactionId>>,
    // ) -> Result<Transaction<'a>, DeclareError> {
    //     match self.declare_inner(global_id.into()).await {
    //         Ok(declared) => Ok(Transaction { controller: self, declared }),
    //         Err(error) => Err(DeclareError::from((self, error))),
    //     }
    // }

    pub(crate) async fn declare_inner(
        &self,
        global_id: Option<TransactionId>,
    ) -> Result<Declared, link::SendError> {
        // To begin transactional work, the transaction controller needs to obtain a transaction
        // identifier from the resource. It does this by sending a message to the coordinator whose
        // body consists of the declare type in a single amqp-value section. Other standard message
        // sections such as the header section SHOULD be ignored.
        let declare = Declare { global_id };
        let message = Message::<Declare>::builder().value(declare).build();
        // This message MUST NOT be sent settled as the sender is REQUIRED to receive and interpret
        // the outcome of the declare from the receiver
        let sendable = Sendable::builder().message(message).settled(false).build();

        let outcome = send_on_control_link(&mut self.inner.borrow_mut(), sendable).await?;
        match outcome.await? {
            DeliveryState::Declared(declared) => Ok(declared),
            DeliveryState::Rejected(rejected) => Err(link::SendError::Rejected(rejected)),
            DeliveryState::Received(_)
            | DeliveryState::Accepted(_)
            | DeliveryState::Released(_)
            | DeliveryState::Modified(_)
            | DeliveryState::TransactionalState(_) => {
                Err(link::SendError::Local(definitions::Error::new(
                    AmqpError::NotAllowed,
                    "Controller is expecting either a Declared outcome or a Rejeccted outcome"
                        .to_string(),
                    None,
                )))
            }
        }
    }

    /// Discharge
    pub(crate) async fn discharge(&self, txn_id: TransactionId, fail: impl Into<Option<bool>>) -> Result<(), link::SendError> {
        let discharge = Discharge {
            txn_id,
            fail: fail.into(),
        };
        // As with the declare message, it is an error if the sender sends the transfer pre-settled.
        let message = Message::<Discharge>::builder().value(discharge).build();
        let sendable = Sendable::builder().message(message).settled(false).build();

        let outcome = send_on_control_link(&mut self.inner.borrow_mut(), sendable).await?;
        match outcome.await? {
            DeliveryState::Accepted(_) => Ok(()),
            DeliveryState::Rejected(rejected) => Err(link::SendError::Rejected(rejected)),
            DeliveryState::Received(_)
            | DeliveryState::Released(_)
            | DeliveryState::Modified(_)
            | DeliveryState::Declared(_)
            | DeliveryState::TransactionalState(_) => {
                Err(link::SendError::Local(definitions::Error::new(
                    AmqpError::NotAllowed,
                    "Controller is expecting either an Accepted outcome or a Rejected outcome"
                        .to_string(),
                    None,
                )))
            }
        }
    }
}

// TODO: implement Drop for controller to drop all non-committed transactions