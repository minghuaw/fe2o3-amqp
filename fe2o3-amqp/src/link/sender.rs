use std::time::Duration;

use tokio::sync::mpsc;

use fe2o3_amqp_types::{
    messaging::{Address, Message, Source},
    performatives::{Disposition},
};

use crate::{session::SessionHandle};

use super::{
    builder::{self, WithoutName, WithoutTarget},
    role,
    sender_link::SenderLink,
    LinkFrame,
    Error,
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
        builder::Builder::new().source(Source::builder().build()) // TODO: where should
    }

    pub async fn attach(
        session: &mut SessionHandle,
        name: impl Into<String>,
        addr: impl Into<Address>,
    ) -> Result<Self, Error> {
        Self::builder()
            .name(name)
            .target(addr)
            .attach(session)
            .await
    }

    pub async fn send(&mut self, message: Message) -> Result<Disposition, Error> {
        todo!()
    }

    pub async fn send_with_timeout(
        &mut self,
        message: Message,
        timeout: impl Into<Duration>,
    ) -> Result<Disposition, Error> {
        todo!()
    }

    pub async fn detach(&mut self) -> Result<(), Error> {
        // detach will send detach with closed=false and wait for remote detach
        // The sender may reattach after fully detached

        todo!()
    }

    pub async fn detach_with_timeout(&mut self, timeout: impl Into<Duration>) -> Result<(), Error> {
        todo!()
    }

    pub async fn close(self) -> Result<(), Error> {
        // Send detach with closed=true and wait for remote closing detach
        // The sender will be dropped after close
        todo!()
    }
}
