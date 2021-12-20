use std::{time::Duration, marker::PhantomData};

use tokio::sync::mpsc;

use fe2o3_amqp_types::{
    messaging::{Address, Message, Source},
    performatives::{Disposition}, definitions::{AmqpError, self, ErrorCondition},
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{session::SessionHandle, endpoint::Link};

use super::{
    builder::{self, WithoutName, WithoutTarget, WithName, WithTarget},
    role,
    sender_link::SenderLink,
    LinkFrame,
    Error, error::DetachError, type_state::{Detached, Attached},
};

pub struct Sender<S> {
    // The SenderLink manages the state
    pub(crate) link: SenderLink,

    // Outgoing mpsc channel to send the Link frames
    pub(crate) outgoing: PollSender<LinkFrame>,
    pub(crate) incoming: ReceiverStream<LinkFrame>,

    // Type state marker
    pub(crate) marker: PhantomData<S>,
}

impl Sender<Detached> {
    pub fn builder() -> builder::Builder<role::Sender, WithoutName, WithoutTarget> {
        builder::Builder::new().source(Source::builder().build()) // TODO: where should
    }

    pub fn modify(self) -> builder::Builder<role::Sender, WithName, WithTarget> {
        todo!()
    }

    // Re-attach the link
    pub async fn reattach(&mut self) -> Result<Sender<Attached>, Error> {
        super::do_attach(&mut self.link, &mut self.outgoing, &mut self.incoming).await?;
        // self.link.on_incoming_attach(remote_attach).await?;

        todo!()
    }
}

impl Sender<Attached> {
    pub async fn attach(
        session: &mut SessionHandle,
        name: impl Into<String>,
        addr: impl Into<Address>,
    ) -> Result<Self, Error> {
        Sender::<Detached>::builder()
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

    /// Detach the link
    /// 
    /// The Sender will send a detach frame with closed field set to false,
    /// and wait for a detach with closed field set to false from the remote peer.
    /// 
    /// # Error
    /// 
    /// If the remote peer sends a detach frame with closed field set to true,
    /// the Sender will re-attach and send a closing detach
    pub async fn detach(mut self) -> Result<Sender<Detached>, DetachError> {
        use futures_util::StreamExt;

        println!(">>> Debug: Sender::detach");
        // TODO: how should disposition be handled?

        // detach will send detach with closed=false and wait for remote detach
        // The sender may reattach after fully detached
        self.link.send_detach(&mut self.outgoing, false, None).await
            .map_err(map_send_detach_error)?;

        // Wait for remote detach
        let frame = self.incoming.next().await
            .ok_or_else(|| detach_error_expecting_frame())?;
        println!(">>> Debug: incoming link frame: {:?}", &frame);
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame())
        };

        if remote_detach.closed {
            // Note that one peer MAY send a closing detach while its partner is 
            // sending a non-closing detach. In this case, the partner MUST 
            // signal that it has closed the link by reattaching and then sending 
            // a closing detach.
            todo!()
        } else {
            self.link.on_incoming_detach(remote_detach).await?;
        }

        Ok(Sender::<Detached> {
            link: self.link,
            outgoing: self.outgoing,
            incoming: self.incoming,
            marker: PhantomData,
        })
    }

    pub async fn detach_with_timeout(&mut self, timeout: impl Into<Duration>) -> Result<(), Error> {
        todo!()
    }

    pub async fn close(mut self) -> Result<(), DetachError> {
        use futures_util::StreamExt;

        // Send detach with closed=true and wait for remote closing detach
        // The sender will be dropped after close
        self.link.send_detach(&mut self.outgoing, true, None).await
            .map_err(map_send_detach_error)?;

        // Wait for remote detach
        let frame = self.incoming.next().await
            .ok_or_else(|| detach_error_expecting_frame())?;
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame())
        };

        if remote_detach.closed {
            self.link.on_incoming_detach(remote_detach).await?;
        } else {
            // Note that one peer MAY send a closing detach while its partner is 
            // sending a non-closing detach. In this case, the partner MUST 
            // signal that it has closed the link by reattaching and then sending 
            // a closing detach.
            todo!()
        }

        Ok(())
    }
}

fn detach_error_expecting_frame() -> DetachError {
    DetachError::from (
        definitions::Error::new(
            AmqpError::IllegalState,
            Some("Expecting remote detach frame".to_string()),
            None
        )
    )
}

fn map_send_detach_error(err: Error) -> DetachError {
    let (condition, description): (ErrorCondition, _) = match err {
        Error::HandleMaxReached => unreachable!(),
        Error::DuplicatedLinkName => unreachable!(),
        Error::AmqpError{ condition, description} => (condition.into(), description),
        Error::LinkError{ condition, description} => (condition.into(), description)
    };
    DetachError {
        is_closed_by_remote: false,
        error: Some(definitions::Error::new(condition, description, None))
    }
}