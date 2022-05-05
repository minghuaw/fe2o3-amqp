use fe2o3_amqp_types::{transaction::{Coordinator, TransactionId, Declared, Declare}, definitions::SenderSettleMode, messaging::Message};

use crate::{link::{sender::SenderInner, SenderFlowState, delivery::UnsettledMessage, Link, role, AttachError, builder::{WithoutName, WithoutTarget}, self}, session::SessionHandle, util::{Uninitialized, Initialized}, Sendable};

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
    pub(crate) declared: D
}

impl Controller<Undeclared> {
    /// Creates a new builder for controller
    pub fn builder() -> link::builder::Builder<role::Sender, Coordinator, WithoutName, WithoutTarget> {
        link::builder::Builder::<role::Sender, Coordinator, WithoutName, WithoutTarget>::new()
    }

    /// Attach the controller
    pub async fn attach<R>(
        session: &mut SessionHandle<R>,
        name: impl Into<String>,
        coordinator: impl Into<Coordinator>
    ) -> Result<Self, AttachError> {
        Self::builder()
            .name(name)
            .coordinator(coordinator)
            .sender_settle_mode(SenderSettleMode::Unsettled)
            .attach(session)
            .await
    }

    /// Declare a transaction
    pub async fn declare(mut self, global_id: Option<TransactionId>) -> Result<Controller<Declared>, DeclareError> {
        // To begin transactional work, the transaction controller needs to obtain a transaction
        // identifier from the resource. It does this by sending a message to the coordinator whose
        // body consists of the declare type in a single amqp-value section. Other standard message
        // sections such as the header section SHOULD be ignored.
        let declare = Declare { global_id };
        let message = Message::<Declare>::builder()
            .value(declare)
            .build();
        // This message MUST NOT be sent settled as the sender is REQUIRED to receive and interpret
        // the outcome of the declare from the receiver
        let sendable = Sendable::builder()
            .message(message)
            .settled(false)
            .build();

        let settlement = self.inner.send(sendable).await
            .map_err(|err| DeclareError::new(self, err))?;
        todo!()
    }
}

impl Controller<Declared> {
    /// Discharge the transaction
    pub async fn discharge(self, fail: bool) -> Result<(), link::Error> {
        todo!()
    }
}