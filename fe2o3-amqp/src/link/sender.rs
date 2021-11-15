use std::time::Duration;

use tokio::sync::mpsc;

use async_trait::async_trait;

use fe2o3_amqp_types::{
    messaging::{Address, Message},
    performatives::{Attach, Detach, Disposition, Flow},
};

use crate::{endpoint, error::EngineError, session::SessionHandle};

use super::{
    builder::{self, WithoutName, WithoutTarget},
    role,
    sender_link::SenderLink,
    LinkFrame,
};

pub struct Sender {
    // The SenderLink manages the state
    pub(crate) link: SenderLink,

    // Outgoing mpsc channel to send the Link frames
    pub(crate) outgoing: mpsc::Sender<LinkFrame>,
    pub(crate) incoming: mpsc::Receiver<LinkFrame>,
}

impl Sender {
    pub fn builder() -> builder::Builder<role::Sender, WithoutName, WithoutTarget> {
        builder::Builder::new()
    }

    pub async fn attach(
        session: &mut SessionHandle,
        name: impl Into<String>,
        addr: impl Into<Address>,
    ) -> Result<Self, EngineError> {
        Self::builder()
            .name(name)
            .target(addr)
            .attach(session)
            .await
    }

    pub async fn send(&mut self, message: Message) -> Result<Disposition, EngineError> {
        todo!()
    }

    pub async fn send_with_timeout(
        &mut self,
        message: Message,
        timeout: Duration,
    ) -> Result<Disposition, EngineError> {
        todo!()
    }

    pub async fn detach(&mut self) -> Result<(), EngineError> {
        todo!()
    }
}
