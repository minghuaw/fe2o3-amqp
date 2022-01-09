use std::{marker::PhantomData, time::Duration, sync::Arc};

use bytes::{Bytes, BytesMut};
use tokio::sync::mpsc;

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, ErrorCondition},
    messaging::{Address, DeliveryState, Message, Source},
    performatives::Disposition,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{
    control::SessionControl,
    endpoint::{Link, Settlement},
    session::{self, SessionHandle}, util::Consumer,
};

use super::{
    builder::{self, WithName, WithTarget, WithoutName, WithoutTarget},
    delivery::{DeliveryFut, Sendable},
    error::DetachError,
    role,
    type_state::{Attached, Detached},
    Error, LinkFrame, LinkHandle, state::{LinkState, LinkFlowState},
};

#[derive(Debug)]
pub struct Sender<S> {
    // The SenderLink manages the state
    pub(crate) link: super::Link<role::Sender, Consumer<Arc<LinkFlowState>>>,
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
            let link_handle = LinkHandle::Sender {
                tx,
                flow_state: self.link.flow_state.producer(),
                // TODO: what else to do during re-attaching
                unsettled: self.link.unsettled.clone(),
                receiver_settle_mode: self.link.rcv_settle_mode.clone(),
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

    pub async fn attach(
        session: &mut SessionHandle,
        name: impl Into<String>,
        addr: impl Into<Address>,
    ) -> Result<Sender<Attached>, Error> {
        Self::builder()
            .name(name)
            .target(addr)
            .attach(session)
            .await
    }
}

impl Sender<Attached> {

    async fn send_inner(&mut self, delivery: Sendable) -> Result<Settlement, Error> {
        use bytes::BufMut;
        use serde::Serialize;
        use serde_amqp::ser::Serializer;

        use crate::endpoint::SenderLink;

        let Sendable {
            message,
            message_format,
            settled,
        } = delivery.into();
        // .try_into().map_err(Into::into)?;

        // serialize message
        let mut payload = BytesMut::new();
        let mut serializer = Serializer::from((&mut payload).writer());
        message.serialize(&mut serializer)?;
        let payload = Bytes::from(payload);

        // send a transfer, checking state will be implemented in SenderLink
        let settlement = self
            .link
            .send_transfer(&mut self.outgoing, payload, message_format, settled, false)
            .await?;
        Ok(settlement)
    }

    pub async fn send<D>(&mut self, delivery: D) -> Result<(), Error>
    where
        D: Into<Sendable>,
        // D: TryInto<Delivery>,
        // D::Error: Into<Error>,
    {
        let settlement = self.send_inner(delivery.into()).await?;

        // If not settled, must wait for outcome
        match settlement {
            Settlement::Settled => Ok(()),
            Settlement::Unsettled { delivery_tag: _, outcome} => {
                let state = outcome.await.map_err(|_| Error::AmqpError {
                    condition: AmqpError::IllegalState,
                    description: Some("Outcome sender is dropped".into()),
                })?;
                match state {
                    DeliveryState::Accepted(_) | DeliveryState::Received(_) => Ok(()),
                    DeliveryState::Rejected(rejected) => Err(Error::Rejected(rejected)),
                    DeliveryState::Released(released) => Err(Error::Released(released)),
                    DeliveryState::Modified(modified) => Err(Error::Modified(modified)),
                }
            }
        }
    }

    pub async fn send_with_timeout(
        &mut self,
        message: Message,
        timeout: impl Into<Duration>,
    ) -> Result<Disposition, Error> {
        todo!()
    }

    pub async fn send_batchable(
        &mut self,
        delivery: impl Into<Sendable>,
    ) -> Result<DeliveryFut, Error> {
        let settlement = self.send_inner(delivery.into()).await?;

        Ok(DeliveryFut::from(settlement))
    }

    pub async fn send_batchable_with_timeout(
        &mut self,
        delivery: impl Into<Sendable>,
        timeout: impl Into<Duration>,
    ) -> Result<DeliveryFut, Error> {
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
    pub async fn detach(self) -> Result<Sender<Detached>, DetachError<Sender<Detached>>> {
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
        match detaching.link
            .send_detach(&mut detaching.outgoing, false, None).await 
        {
            Ok(_) => {},
            Err(e) => {
                return Err(map_send_detach_error(e, detaching))
            }
        };

        // Wait for remote detach
        let frame= match detaching.incoming.next().await {
            Some(frame) => frame,
            None => {
                return Err(detach_error_expecting_frame(detaching))
            }
        };

        println!(">>> Debug: incoming link frame: {:?}", &frame);
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame(detaching)),
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

            // A peer closes a link by sending the detach frame with the handle for the 
            // specified link, and the closed flag set to true. The partner will destroy 
            // the corresponding link endpoint, and reply with its own detach frame with 
            // the closed flag set to true.
            return Err(DetachError {
                link: None,
                is_closed_by_remote: true,
                error: None,
            });
        } else {
            match detaching.link.on_incoming_detach(remote_detach).await {
                Ok(_) => {}
                Err(e) => return Err(DetachError {
                    link: Some(detaching),
                    is_closed_by_remote: false,
                    error: Some(e)
                })
            }
        }

        // TODO: de-allocate link from session
        match detaching
            .session
            .send(SessionControl::DeallocateLink(detaching.link.name.clone()))
            .await
        {
            Ok(_) => {},
            Err(e) => return Err(map_send_detach_error(e, detaching))
        }

        Ok(detaching)
    }

    pub async fn detach_with_timeout(&mut self, timeout: impl Into<Duration>) -> Result<(), Error> {
        todo!()
    }

    pub async fn close(self) -> Result<(), DetachError<Sender<Detached>>> {
        use futures_util::StreamExt;

        let mut detaching = Sender::<Detached> {
            link: self.link,
            buffer_size: self.buffer_size,
            session: self.session,
            outgoing: self.outgoing,
            incoming: self.incoming,
            marker: PhantomData,
        };

        // Send detach with closed=true and wait for remote closing detach
        // The sender will be dropped after close
        match detaching.link
            .send_detach(&mut detaching.outgoing, true, None)
            .await
        {
            Ok(_) => {},
            Err(e) => return Err(map_send_detach_error(e, detaching))
        }

        // Wait for remote detach
        let frame = match detaching
            .incoming
            .next()
            .await {
                Some(frame) => frame,
                None => return Err(detach_error_expecting_frame(detaching))
            };
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame(detaching)),
        };

        if remote_detach.closed {
            // If the remote detach contains an error, the error will be propagated
            // back by `on_incoming_detach`
            match detaching.link.on_incoming_detach(remote_detach).await {
                Ok(_) => {},
                Err(e) => return Err(DetachError {
                    link: Some(detaching),
                    is_closed_by_remote: false,
                    error: Some(e)
                })
            }
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
        match detaching.session
            .send(SessionControl::DeallocateLink(detaching.link.name.clone()))
            .await
            // .map_err(|e| map_send_detach_error)?;
        {
            Ok(_) => {},
            Err(e) => return Err(map_send_detach_error(e, detaching))
        }

        Ok(())
    }
}

fn detach_error_expecting_frame<L>(link: L) -> DetachError<L> {
    let error = definitions::Error::new(
        AmqpError::IllegalState,
        Some("Expecting remote detach frame".to_string()),
        None,
    );

    DetachError {
        link: Some(link),
        is_closed_by_remote: false,
        error: Some(error)
    }
}

fn map_send_detach_error<L>(err: impl Into<Error>, link: L) -> DetachError<L> {
    let (condition, description): (ErrorCondition, _) = match err.into() {
        Error::AmqpError {
            condition,
            description,
        } => (condition.into(), description),
        Error::LinkError {
            condition,
            description,
        } => (condition.into(), description),
        Error::HandleMaxReached
        | Error::DuplicatedLinkName
        | Error::ParseError
        | Error::Rejected(_)
        | Error::Released(_)
        | Error::Modified(_) => unreachable!(),
    };
    DetachError {
        link: Some(link),
        is_closed_by_remote: false,
        error: Some(definitions::Error::new(condition, description, None)),
    }
}
