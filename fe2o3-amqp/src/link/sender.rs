use std::time::Duration;

use tokio::sync::mpsc;

use async_trait::async_trait;

use fe2o3_amqp_types::{messaging::Message, performatives::{Attach, Detach, Disposition, Flow}};

use crate::{endpoint, error::EngineError, session::SessionHandle};

use super::{LinkFrame, builder::{self, WithoutName, WithoutTarget}, role};

pub struct Sender<L> {
    link: L
}

impl<L: endpoint::SenderLink> Sender<L> {
    pub fn builder() -> builder::Builder<role::Sender, WithoutName, WithoutTarget> {
        todo!()
    }

    pub async fn attach(
        session: &mut SessionHandle,
        name: impl Into<String>,
    ) -> Result<Self, EngineError> {
        todo!()
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

/// TODO: impl `futures_util::io::IntoSink`
pub struct SenderSink {}
