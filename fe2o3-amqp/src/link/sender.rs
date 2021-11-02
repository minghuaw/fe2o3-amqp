use std::time::Duration;

use tokio::sync::mpsc;

use async_trait::async_trait;

use fe2o3_amqp_types::{messaging::Message, performatives::{Attach, Detach, Disposition, Flow}};

use crate::{endpoint, error::EngineError, session::SessionHandle};

use super::{LinkFrame, builder::{self, WithoutName, WithoutTarget}, role};

/// Manages the link state
pub struct SenderLink {

}

#[async_trait]
impl endpoint::Link for SenderLink {
    type Error = EngineError;
    
    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_flow(&mut self, flow: Flow) -> Result<(), Self::Error> {
        todo!()
    }

    // Only the receiver is supposed to receive incoming Transfer frame

    async fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_attach(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_flow(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_disposition(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_detach(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

#[async_trait]
impl endpoint::SenderLink for SenderLink {
    async fn send_transfer(&mut self, writer: &mut mpsc::Sender<LinkFrame>) -> Result<(), <Self as endpoint::Link>::Error> {
        todo!()
    }
}

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
