use std::time::Duration;

use tokio::sync::mpsc;

use fe2o3_amqp_types::{
    messaging::{Address, Message, Source},
    performatives::{Disposition}, definitions::{AmqpError, self},
};
use tokio_util::sync::PollSender;

use crate::{session::SessionHandle, endpoint::Link};

use super::{
    builder::{self, WithoutName, WithoutTarget},
    role,
    sender_link::SenderLink,
    LinkFrame,
    Error, error::DetachError,
};

pub struct Sender {
    // The SenderLink manages the state
    pub(crate) link: SenderLink,

    // Outgoing mpsc channel to send the Link frames
    pub(crate) outgoing: PollSender<LinkFrame>,
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

    pub async fn detach(&mut self) -> Result<(), DetachError> {
        println!(">>> Debug: Sender::detach");
        // TODO: how should disposition be handled?

        // detach will send detach with closed=false and wait for remote detach
        // The sender may reattach after fully detached
        self.link.send_detach(&mut self.outgoing, false, None).await?;

        // Wait for remote detach
        let frame = self.incoming.recv().await
            .ok_or_else(|| detach_error_expecting_frame())?;
        println!(">>> Debug: incoming link frame: {:?}", &frame);
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame())
        };

        if remote_detach.closed {
            // Remote peer is closing while we sent a non-closing detach
            // uamqp implements re-attaching and then send closing detach
            todo!()
        } else {
            self.link.on_incoming_detach(remote_detach).await?;
        }
        Ok(())
    }

    pub async fn detach_with_timeout(&mut self, timeout: impl Into<Duration>) -> Result<(), Error> {
        todo!()
    }

    pub async fn close(mut self) -> Result<(), DetachError> {
        // Send detach with closed=true and wait for remote closing detach
        // The sender will be dropped after close
        self.link.send_detach(&mut self.outgoing, true, None).await?;

        // Wait for remote detach
        let frame = self.incoming.recv().await
            .ok_or_else(|| detach_error_expecting_frame())?;
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame())
        };

        if remote_detach.closed {
            self.link.on_incoming_detach(remote_detach).await?;
        } else {
            todo!()
        }

        Ok(())
    }
}

fn detach_error_expecting_frame() -> DetachError {
    DetachError::new (
        AmqpError::IllegalState,
        Some("Expecting remote detach frame".to_string()),
        None
    )
}