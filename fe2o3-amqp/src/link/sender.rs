use std::{marker::PhantomData, time::Duration};

use tokio::sync::mpsc;

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, ErrorCondition},
    messaging::{Address, Message, Source},
    performatives::Disposition,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{
    control::SessionControl,
    endpoint::Link,
    session::{self, SessionHandle},
};

use super::{
    builder::{self, WithName, WithTarget, WithoutName, WithoutTarget},
    error::DetachError,
    role,
    sender_link::SenderLink,
    type_state::{Attached, Detached},
    Error, LinkFrame, LinkHandle,
};

pub struct Sender<S> {
    // The SenderLink manages the state
    pub(crate) link: SenderLink,
    pub(crate) buffer_size: usize,

    // Control sender to the session
    pub(crate) session: mpsc::Sender<SessionControl>,

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
    pub async fn reattach(self, session: &mut SessionHandle) -> Result<Sender<Attached>, Error> {
        self.reattach_inner(session.control.clone()).await
    }

    async fn reattach_inner(
        mut self,
        mut session_control: mpsc::Sender<SessionControl>,
    ) -> Result<Sender<Attached>, Error> {
        // May need to re-allocate output handle
        if self.link.output_handle.is_none() {
            let (tx, incoming) = mpsc::channel(self.buffer_size);
            let link_handle = LinkHandle {
                tx,
                flow_state: self.link.flow_state.producer(),
            };
            self.incoming = ReceiverStream::new(incoming);
            let handle =
                session::allocate_link(&mut session_control, self.link.name.clone(), link_handle)
                    .await?;
            self.link.output_handle = Some(handle);
        }

        super::do_attach(&mut self.link, &mut self.outgoing, &mut self.incoming).await?;

        Ok(Sender::<Attached> {
            link: self.link,
            buffer_size: self.buffer_size,
            session: self.session,
            outgoing: self.outgoing,
            incoming: self.incoming,
            marker: PhantomData,
        })
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

    pub async fn send(&mut self, message: Message) -> Result<(), Error> {
        // send a transfer, checking state will be implemented in SenderLink

        // depending on

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
    pub async fn detach(self) -> Result<Sender<Detached>, DetachError> {
        use futures_util::StreamExt;

        println!(">>> Debug: Sender::detach");
        let mut detaching = Sender::<Detached> {
            link: self.link,
            buffer_size: self.buffer_size,
            session: self.session,
            outgoing: self.outgoing,
            incoming: self.incoming,
            marker: PhantomData,
        };

        // TODO: how should disposition be handled?

        // detach will send detach with closed=false and wait for remote detach
        // The sender may reattach after fully detached
        detaching
            .link
            .send_detach(&mut detaching.outgoing, false, None)
            .await
            .map_err(map_send_detach_error)?;

        // Wait for remote detach
        let frame = detaching
            .incoming
            .next()
            .await
            .ok_or_else(|| detach_error_expecting_frame())?;
        println!(">>> Debug: incoming link frame: {:?}", &frame);
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame()),
        };

        if remote_detach.closed {
            // Note that one peer MAY send a closing detach while its partner is
            // sending a non-closing detach. In this case, the partner MUST
            // signal that it has closed the link by reattaching and then sending
            // a closing detach.
            let session_control = detaching.session.clone();
            let reattached = match detaching.reattach_inner(session_control).await {
                Ok(sender) => sender,
                Err(e) => {
                    //
                    println!("{:?}", e);
                    todo!()
                }
            };

            reattached.close().await?;

            return Err(DetachError {
                is_closed_by_remote: true,
                error: None,
            });
        } else {
            detaching.link.on_incoming_detach(remote_detach).await?;
        }

        // TODO: de-allocate link from session
        detaching
            .session
            .send(SessionControl::DeallocateLink(detaching.link.name.clone()))
            .await
            .map_err(map_send_detach_error)?;

        Ok(detaching)
    }

    pub async fn detach_with_timeout(&mut self, timeout: impl Into<Duration>) -> Result<(), Error> {
        todo!()
    }

    pub async fn close(mut self) -> Result<(), DetachError> {
        use futures_util::StreamExt;

        // Send detach with closed=true and wait for remote closing detach
        // The sender will be dropped after close
        self.link
            .send_detach(&mut self.outgoing, true, None)
            .await
            .map_err(map_send_detach_error)?;

        // Wait for remote detach
        let frame = self
            .incoming
            .next()
            .await
            .ok_or_else(|| detach_error_expecting_frame())?;
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame()),
        };

        if remote_detach.closed {
            // If the remote detach contains an error, the error will be propagated
            // back by `on_incoming_detach`
            self.link.on_incoming_detach(remote_detach).await?;
        } else {
            // Note that one peer MAY send a closing detach while its partner is
            // sending a non-closing detach. In this case, the partner MUST
            // signal that it has closed the link by reattaching and then sending
            // a closing detach.

            // Probably something like below
            // 1. wait for incoming attach
            // 2. send back attach
            // 3. wait for incoming closing detach
            // 4. detach

            todo!()
        }

        // TODO: de-allocate link from session
        self.session
            .send(SessionControl::DeallocateLink(self.link.name.clone()))
            .await
            .map_err(map_send_detach_error)?;

        Ok(())
    }
}

fn detach_error_expecting_frame() -> DetachError {
    DetachError::from(definitions::Error::new(
        AmqpError::IllegalState,
        Some("Expecting remote detach frame".to_string()),
        None,
    ))
}

fn map_send_detach_error(err: impl Into<Error>) -> DetachError {
    let (condition, description): (ErrorCondition, _) = match err.into() {
        Error::HandleMaxReached => unreachable!(),
        Error::DuplicatedLinkName => unreachable!(),
        Error::AmqpError {
            condition,
            description,
        } => (condition.into(), description),
        Error::LinkError {
            condition,
            description,
        } => (condition.into(), description),
    };
    DetachError {
        is_closed_by_remote: false,
        error: Some(definitions::Error::new(condition, description, None)),
    }
}

pub struct SendFut {}
