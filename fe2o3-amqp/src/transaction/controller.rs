use fe2o3_amqp_types::transaction::Coordinator;

use crate::{link::{sender::SenderInner, SenderFlowState, delivery::UnsettledMessage, Link, role, AttachError, builder::{WithoutName, WithoutTarget}, self}, session::SessionHandle};

pub(crate) type ControlLink = Link<role::Sender, Coordinator, SenderFlowState, UnsettledMessage>;

/// Transaction controller
#[derive(Debug)]
pub struct Controller {
    pub(crate) inner: SenderInner<ControlLink>,
}

impl Controller {
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
            .attach(session)
            .await
    }
}