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

use super::DeclareError;

pub(crate) type ControlLink = Link<role::Sender, Coordinator, SenderFlowState, UnsettledMessage>;

/// Zero-sized type state representing a controller that has not declared a transaction
#[derive(Debug)]
pub struct Undeclared {}

/// Transaction controller
///
/// # Type parameter `S`
///
/// This is a type state with two possible values
///
/// 1. [`Undeclared`] representing a controller that
/// 2. [`Declared`]
#[derive(Debug)]
pub struct Controller<D> {
    pub(crate) inner: SenderInner<ControlLink>,
    pub(crate) declared: D,
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

impl<D> Controller<D> {
    /// Close the control link with error
    pub async fn close_with_error(
        &mut self,
        error: definitions::Error,
    ) -> Result<(), link::DetachError> {
        self.inner.close_with_error(Some(error)).await
    }

    /// Close the link
    pub async fn close(&mut self) -> Result<(), link::DetachError> {
        self.inner.close_with_error(None).await
    }
}

impl Controller<Undeclared> {
    /// Creates a new builder for controller
    pub fn builder() -> link::builder::Builder<role::Sender, Coordinator, WithoutName, WithoutTarget>
    {
        link::builder::Builder::<role::Sender, Coordinator, WithoutName, WithoutTarget>::new()
    }

    /// Attach the controller
    pub async fn attach<R>(
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

    /// Declare a transaction
    pub async fn declare(
        mut self,
        global_id: impl Into<Option<TransactionId>>,
    ) -> Result<Controller<Declared>, DeclareError> {
        match self.declare_inner(global_id.into()).await {
            Ok(declared) => Ok(Controller {
                inner: self.inner,
                declared,
            }),
            Err(error) => Err(DeclareError::from((self, error))),
        }
    }

    async fn declare_inner(
        &mut self,
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

        let outcome = send_on_control_link(&mut self.inner, sendable).await?;
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
}

impl Controller<Declared> {
    /// Gets the transaction ID
    pub fn txn_id(&self) -> &TransactionId {
        &self.declared.txn_id
    }

    /// Commit the transaction
    ///
    /// This will send a [`Discharge`] with the `fail` field set to false.
    ///
    /// If the coordinator is unable to complete the discharge, the coordinator MUST convey the
    /// error to the controller as a transaction-error
    pub async fn commit(&mut self) -> Result<(), link::SendError> {
        self.discharge(false).await
    }

    /// Rollback the transaction
    ///
    /// This will send a [`Discharge`] with the `fail` field set to true
    ///
    /// If the coordinator is unable to complete the discharge, the coordinator MUST convey the
    /// error to the controller as a transaction-error
    pub async fn rollback(&mut self) -> Result<(), link::SendError> {
        self.discharge(true).await
    }

    /// Discharge
    async fn discharge(&mut self, fail: impl Into<Option<bool>>) -> Result<(), link::SendError> {
        let discharge = Discharge {
            txn_id: self.declared.txn_id.clone(),
            fail: fail.into(),
        };
        // As with the declare message, it is an error if the sender sends the transfer pre-settled.
        let message = Message::<Discharge>::builder().value(discharge).build();
        let sendable = Sendable::builder().message(message).settled(false).build();

        let outcome = send_on_control_link(&mut self.inner, sendable).await?;
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
