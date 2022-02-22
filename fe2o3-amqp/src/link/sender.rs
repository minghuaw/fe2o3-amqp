use std::{marker::PhantomData, sync::Arc, time::Duration};

use bytes::BytesMut;
use futures_util::StreamExt;
use tokio::{sync::mpsc, time::{error::Elapsed, timeout}};

use fe2o3_amqp_types::{
    definitions::{self, AmqpError},
    messaging::{message::__private::Serializable, Address, DeliveryState, Message, Source},
    performatives::Disposition,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{
    control::SessionControl,
    endpoint::{Link, Settlement},
    link::error::{detach_error_expecting_frame, map_send_detach_error},
    session::{self, SessionHandle},
    util::Consumer,
};

use super::{
    builder::{self, WithoutName, WithoutTarget},
    delivery::{DeliveryFut, Sendable, UnsettledMessage},
    error::DetachError,
    role,
    state::LinkFlowState,
    type_state::{Attached, Detached},
    Error, LinkFrame, LinkHandle,
};

type SenderFlowState = LinkFlowState<role::Sender>;
type SenderLink = super::Link<role::Sender, Consumer<Arc<SenderFlowState>>, UnsettledMessage>;

#[derive(Debug)]
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

    // // Re-attach the link
    // pub async fn reattach(self, session: &mut SessionHandle) -> Result<Sender<Attached>, Error> {
    //     self.reattach_inner(session.control.clone()).await
    // }

    async fn reattach_inner(
        mut self,
        mut session_control: mpsc::Sender<SessionControl>,
    ) -> Result<Sender<Attached>, DetachError<Self>> {
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
            let handle = match session::allocate_link(
                &mut session_control,
                self.link.name.clone(),
                link_handle,
            )
            .await
            {
                Ok(handle) => handle,
                Err(err) => {
                    return Err(DetachError {
                        link: Some(self),
                        is_closed_by_remote: false,
                        error: Some(definitions::Error::from(err)),
                    });
                }
            };
            self.link.output_handle = Some(handle);
        }

        if let Err(err) =
            super::do_attach(&mut self.link, &mut self.outgoing, &mut self.incoming).await
        {
            return Err(map_send_detach_error(err, self));
        }

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
    fn into_detached(self) -> Sender<Detached> {
        Sender::<Detached> {
            link: self.link,
            buffer_size: self.buffer_size,
            session: self.session,
            outgoing: self.outgoing,
            incoming: self.incoming,
            marker: PhantomData,
        }
    }

    async fn send_inner<T>(&mut self, sendable: Sendable<T>) -> Result<Settlement, Error>
    where
        T: serde::Serialize,
    {
        use bytes::BufMut;
        use serde::Serialize;
        use serde_amqp::ser::Serializer;

        use crate::endpoint::SenderLink;

        let Sendable {
            message,
            message_format,
            settled,
        } = sendable.into();
        // .try_into().map_err(Into::into)?;

        // serialize message
        let mut payload = BytesMut::new();
        let mut serializer = Serializer::from((&mut payload).writer());
        Serializable(message).serialize(&mut serializer)?;
        // let payload = BytesMut::from(payload);
        let payload = payload.freeze();

        // send a transfer, checking state will be implemented in SenderLink
        let detached_fut = self.incoming.next();
        let settlement = self
            .link
            .send_transfer(
                &mut self.outgoing,
                detached_fut,
                payload,
                message_format,
                settled,
                false,
            )
            .await?;
        Ok(settlement)
    }

    pub async fn send<T: serde::Serialize>(&mut self, sendable: impl Into<Sendable<T>>) -> Result<(), Error>
    {
        let settlement = self.send_inner(sendable.into()).await?;

        // If not settled, must wait for outcome
        match settlement {
            Settlement::Settled | Settlement::Drained => Ok(()),
            Settlement::Unsettled {
                delivery_tag: _,
                outcome,
            } => {
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

    pub async fn send_with_timeout<T: serde::Serialize>(
        &mut self,
        sendable: impl Into<Sendable<T>>,
        duration: impl Into<Duration>,
    ) -> Result<Result<(), Error>, Elapsed> {
        timeout(duration.into(), self.send(sendable)).await
    }

    pub async fn send_batchable<T: serde::Serialize>(
        &mut self,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<DeliveryFut, Error> {
        let settlement = self.send_inner(sendable.into()).await?;

        Ok(DeliveryFut::from(settlement))
    }

    pub async fn send_batchable_with_timeout<T: serde::Serialize>(
        &mut self,
        sendable: impl Into<Sendable<T>>,
        duration: impl Into<Duration>,
    ) -> Result<Result<DeliveryFut, Error>, Elapsed> {
        timeout(duration.into(), self.send_batchable(sendable)).await
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
        println!(">>> Debug: Sender::detach");
        let mut detaching = self.into_detached();

        // TODO: how should disposition be handled?

        // detach will send detach with closed=false and wait for remote detach
        // The sender may reattach after fully detached
        match detaching
            .link
            .send_detach(&mut detaching.outgoing, false, None)
            .await
        {
            Ok(_) => {}
            Err(e) => return Err(map_send_detach_error(e, detaching)),
        };

        // Wait for remote detach
        let frame = match detaching.incoming.next().await {
            Some(frame) => frame,
            None => return Err(detach_error_expecting_frame(detaching)),
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
            let reattached = detaching.reattach_inner(session_control).await?;

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
                Err(e) => {
                    return Err(DetachError {
                        link: Some(detaching),
                        is_closed_by_remote: false,
                        error: Some(e),
                    })
                }
            }
        }

        // TODO: de-allocate link from session
        match detaching
            .session
            .send(SessionControl::DeallocateLink(detaching.link.name.clone()))
            .await
        {
            Ok(_) => {}
            Err(e) => return Err(map_send_detach_error(e, detaching)),
        }

        Ok(detaching)
    }

    pub async fn detach_with_timeout(self, duration: impl Into<Duration>) -> Result<Result<Sender<Detached>, DetachError<Sender<Detached>>>, Elapsed> {
        timeout(duration.into(), self.detach()).await
    }

    pub async fn close(self) -> Result<(), DetachError<Sender<Detached>>> {
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
        match detaching
            .link
            .send_detach(&mut detaching.outgoing, true, None)
            .await
        {
            Ok(_) => {}
            Err(e) => return Err(map_send_detach_error(e, detaching)),
        }

        // Wait for remote detach
        let frame = match detaching.incoming.next().await {
            Some(frame) => frame,
            None => return Err(detach_error_expecting_frame(detaching)),
        };
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame(detaching)),
        };

        let detaching = if remote_detach.closed {
            // If the remote detach contains an error, the error will be propagated
            // back by `on_incoming_detach`
            match detaching.link.on_incoming_detach(remote_detach).await {
                Ok(_) => detaching,
                Err(e) => {
                    return Err(DetachError {
                        link: Some(detaching),
                        is_closed_by_remote: false,
                        error: Some(e),
                    })
                }
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

            let session_control = detaching.session.clone();
            let reattached = detaching.reattach_inner(session_control).await?;
            let mut detaching = reattached.into_detached();
            let frame = match detaching.incoming.next().await {
                Some(frame) => frame,
                None => return Err(detach_error_expecting_frame(detaching)),
            };

            // TODO: is checking closing still necessary?
            let _remote_detach = match frame {
                LinkFrame::Detach(detach) => detach,
                _ => return Err(detach_error_expecting_frame(detaching)),
            };
            match detaching
                .link
                .send_detach(&mut detaching.outgoing, true, None)
                .await
            {
                Ok(_) => detaching,
                Err(err) => return Err(map_send_detach_error(err, detaching)),
            }
        };

        // TODO: de-allocate link from session
        match detaching
            .session
            .send(SessionControl::DeallocateLink(detaching.link.name.clone()))
            .await
        {
            Ok(_) => {}
            Err(e) => return Err(map_send_detach_error(e, detaching)),
        }

        Ok(())
    }
}
