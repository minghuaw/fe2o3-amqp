//! Implementation of AMQP1.0 sender

use std::time::Duration;

use bytes::BytesMut;
use futures_util::StreamExt;
use tokio::{
    sync::mpsc,
    time::{error::Elapsed, timeout},
};

use fe2o3_amqp_types::{
    definitions::{self, AmqpError},
    messaging::{message::__private::Serializable, Address, DeliveryState, Target},
    performatives::Detach,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{
    control::SessionControl,
    endpoint::{self, LinkDetach, LinkExt, Settlement},
    link::error::detach_error_expecting_frame,
    session::{self, SessionHandle},
};

use super::{
    builder::{self, WithoutName, WithoutTarget},
    delivery::{DeliveryFut, Sendable},
    error::{AttachError, DetachError},
    role, ArcSenderUnsettledMap, Error, LinkFrame, LinkRelay, SendError, SenderFlowState,
    SenderLink,
};

/// An AMQP1.0 sender
///
/// # Attach a new sender with default configurations
///
/// ```rust, ignore
/// let sender = Sender::attach(
///     &mut session,           // mutable reference to SessionHandle
///     "rust-sender-link-1",   // link name
///     "q1"                    // Target address
/// ).await.unwrap();
/// ```
///
/// ## Default configuration
///
/// | Field | Default Value |
/// |-------|---------------|
/// |`name`|`String::default()`|
/// |`snd_settle_mode`|`SenderSettleMode::Mixed`|
/// |`rcv_settle_mode`|`ReceiverSettleMode::First`|
/// |`source`|`Some(Source)` |
/// |`target`| `None` |
/// |`initial_delivery_count`| `0` |
/// |`max_message_size`| `None` |
/// |`offered_capabilities`| `None` |
/// |`desired_capabilities`| `None` |
/// |`Properties`| `None` |
/// |`buffer_size`| `u16::MAX` |
/// |`role`| `role::Sender` |
///
/// # Customize configuration with [`builder::Builder`]
///
/// ```rust, ignore
/// let mut sender = Sender::builder()
///     .name("rust-sender-link-1")
///     .target("q1")
///     .sender_settle_mode(SenderSettleMode::Mixed)
///     .attach(&mut session)
///     .await
///     .unwrap();
/// ```
pub struct Sender {
    pub(crate) inner: SenderInner<SenderLink>,
}

impl std::fmt::Debug for Sender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sender")
            // .field("inner", &self.inner)
            .finish()
    }
}

impl Sender {
    /// Creates a builder for [`Sender`] link
    pub fn builder() -> builder::Builder<role::Sender, Target, WithoutName, WithoutTarget> {
        builder::Builder::<role::Sender, Target, _, _>::new()
    }

    /// Attach the sender link to a session with default configuration
    ///
    /// ## Default configuration
    ///
    /// | Field | Default Value |
    /// |-------|---------------|
    /// |`name`|`String::default()`|
    /// |`snd_settle_mode`|`SenderSettleMode::Mixed`|
    /// |`rcv_settle_mode`|`ReceiverSettleMode::First`|
    /// |`source`|`Some(Source)` |
    /// |`target`| `None` |
    /// |`initial_delivery_count`| `0` |
    /// |`max_message_size`| `None` |
    /// |`offered_capabilities`| `None` |
    /// |`desired_capabilities`| `None` |
    /// |`Properties`| `None` |
    /// |`buffer_size`| `u16::MAX` |
    /// |`role`| `role::Sender` |
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let sender = Sender::attach(
    ///     &mut session,           // mutable reference to SessionHandle
    ///     "rust-sender-link-1",   // link name
    ///     "q1"                    // Target address
    /// ).await.unwrap();
    /// ```
    ///
    pub async fn attach<R>(
        session: &mut SessionHandle<R>,
        name: impl Into<String>,
        addr: impl Into<Address>,
    ) -> Result<Sender, AttachError> {
        Self::builder()
            .name(name)
            .target(addr)
            .attach(session)
            .await
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
    pub async fn detach(&mut self) -> Result<(), DetachError> {
        self.inner.detach_with_error(None).await
    }

    /// Detach the link with an error
    pub async fn detach_with_error(
        &mut self,
        error: definitions::Error,
    ) -> Result<(), DetachError> {
        self.inner.detach_with_error(Some(error)).await
    }

    /// Detach the link with a timeout
    pub async fn detach_with_timeout(
        &mut self,
        duration: Duration,
    ) -> Result<Result<(), DetachError>, Elapsed> {
        timeout(duration, self.detach()).await
    }

    /// Close the link.
    ///
    /// This will set the `closed` field in the Detach performative to true
    pub async fn close(&mut self) -> Result<(), DetachError> {
        self.inner.close_with_error(None).await
    }

    /// Detach the link with an error
    pub async fn close_with_error(&mut self, error: definitions::Error) -> Result<(), DetachError> {
        self.inner.close_with_error(Some(error)).await
    }

    /// Send a message and wait for acknowledgement (disposition)
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// sender.send("hello").await.unwrap();
    /// ```
    pub async fn send<T: serde::Serialize>(
        &mut self,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<(), SendError> {
        let settlement = self.inner.send(sendable.into()).await?;

        // If not settled, must wait for outcome
        match settlement {
            Settlement::Settled => Ok(()),
            Settlement::Unsettled {
                _delivery_tag: _,
                outcome,
            } => match outcome.await? {
                DeliveryState::Accepted(_) | DeliveryState::Received(_) => Ok(()),
                DeliveryState::Rejected(rejected) => Err(SendError::Rejected(rejected)),
                DeliveryState::Released(released) => Err(SendError::Released(released)),
                DeliveryState::Modified(modified) => Err(SendError::Modified(modified)),
                #[cfg(feature = "transaction")]
                DeliveryState::Declared(_) | DeliveryState::TransactionalState(_) => {
                    Err(SendError::not_implemented(None))
                }
            },
        }
    }

    /// Send a message and wait for acknowledgement (disposition) with a timeout.
    ///
    /// This simply wraps [`send`](#method.send) inside a [`tokio::time::timeout`]
    pub async fn send_with_timeout<T: serde::Serialize>(
        &mut self,
        sendable: impl Into<Sendable<T>>,
        duration: Duration,
    ) -> Result<Result<(), SendError>, Elapsed> {
        timeout(duration, self.send(sendable)).await
    }

    /// Send a message without waiting for the acknowledgement.
    ///
    /// This will set the batchable field of the `Transfer` performative to true.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let fut = sender.send_batchable("HELLO AMQP").await.unwrap();
    /// let result = fut.await;
    /// println!("fut {:?}", result);
    /// ```
    pub async fn send_batchable<T: serde::Serialize>(
        &mut self,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<DeliveryFut<Result<(), SendError>>, SendError> {
        let settlement = self.inner.send(sendable.into()).await?;

        Ok(DeliveryFut::from(settlement))
    }

    // /// Send a message without waiting for the acknowledgement with a timeout.
    // ///
    // /// This will set the batchable field of the `Transfer` performative to true.
    // pub async fn send_batchable_with_timeout<T: serde::Serialize>(
    //     &mut self,
    //     sendable: impl Into<Sendable<T>>,
    //     duration: Duration,
    // ) -> Result<Timeout<DeliveryFut<Result<(), SendError>>>, SendError> {
    //     let fut = self.send_batchable(sendable).await?;
    //     Ok(timeout(duration, fut))
    // }
}

/// This is so that the transaction controller can re-use
/// the sender
#[derive(Debug)]
pub(crate) struct SenderInner<L: endpoint::SenderLink> {
    // The SenderLink manages the state
    pub(crate) link: L,
    pub(crate) buffer_size: usize,

    // Control sender to the session
    pub(crate) session: mpsc::Sender<SessionControl>,

    // Outgoing mpsc channel to send the Link frames
    pub(crate) outgoing: PollSender<LinkFrame>,
    pub(crate) incoming: ReceiverStream<LinkFrame>,
}

impl<L: endpoint::SenderLink> Drop for SenderInner<L> {
    fn drop(&mut self) {
        if let Some(handle) = self.link.output_handle() {
            if let Some(sender) = self.outgoing.get_ref() {
                let detach = Detach {
                    handle: handle.clone().into(),
                    closed: true,
                    error: None,
                };
                let _ = sender.try_send(LinkFrame::Detach(detach));
            }
        }
    }
}

impl SenderInner<SenderLink> {
    #[inline]
    pub async fn detach_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<(), DetachError> {
        // let mut detaching = self.into_detached();

        // TODO: how should disposition be handled?

        // detach will send detach with closed=false and wait for remote detach
        // The sender may reattach after fully detached
        if let Err(e) = self
            .link
            .send_detach(&mut self.outgoing, false, error)
            .await
        {
            return Err(DetachError::new(false, Some(e)));
        };

        // session::deallocate_link(&mut self.session, output_handle).await?;

        // Wait for remote detach
        let frame = match self.incoming.next().await {
            Some(frame) => frame,
            None => return Err(detach_error_expecting_frame()),
        };

        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame()),
        };

        if remote_detach.closed {
            // Note that one peer MAY send a closing detach while its partner is
            // sending a non-closing detach. In this case, the partner MUST
            // signal that it has closed the link by reattaching and then sending
            // a closing detach.
            let session_control = self.session.clone();
            self.reattach_inner(session_control).await?;

            self.close_with_error(None).await?;

            // A peer closes a link by sending the detach frame with the handle for the
            // specified link, and the closed flag set to true. The partner will destroy
            // the corresponding link endpoint, and reply with its own detach frame with
            // the closed flag set to true.
            return Err(DetachError {
                is_closed_by_remote: true,
                error: None,
            });
        } else {
            match self.link.on_incoming_detach(remote_detach).await {
                Ok(_) => {}
                Err(e) => {
                    return Err(DetachError {
                        is_closed_by_remote: false,
                        error: Some(e),
                    })
                }
            }
        }

        Ok(())
    }
}

impl<L> SenderInner<L>
where
    L: endpoint::SenderLink<
            Error = Error,
            AttachError = AttachError,
            DetachError = definitions::Error,
        > + LinkExt<FlowState = SenderFlowState, Unsettled = ArcSenderUnsettledMap>,
{
    async fn reattach_inner(
        &mut self,
        mut session_control: mpsc::Sender<SessionControl>,
    ) -> Result<&mut Self, DetachError> {
        // May need to re-allocate output handle
        if self.link.output_handle().is_none() {
            let (tx, incoming) = mpsc::channel(self.buffer_size);
            let link_handle = LinkRelay::Sender {
                tx,
                output_handle: (),
                flow_state: self.link.flow_state().producer(),
                // TODO: what else to do during re-attaching
                unsettled: self.link.unsettled().clone(),
                receiver_settle_mode: self.link.rcv_settle_mode().clone(),
                // state_code: self.link.state_code.clone(),
            };
            self.incoming = ReceiverStream::new(incoming);
            let handle = match session::allocate_link(
                &mut session_control,
                self.link.name().into(),
                link_handle,
            )
            .await
            {
                Ok(handle) => handle,
                Err(err) => {
                    return Err(DetachError {
                        is_closed_by_remote: false,
                        error: Some(definitions::Error::from(err)),
                    });
                }
            };
            *self.link.output_handle_mut() = Some(handle);
        }

        if let Err(_err) =
            super::do_attach(&mut self.link, &mut self.outgoing, &mut self.incoming).await
        {
            let err = definitions::Error::new(AmqpError::IllegalState, None, None);
            return Err(DetachError::new(false, Some(err)));
        }

        Ok(self)
    }

    pub async fn close_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<(), DetachError> {
        // Send detach with closed=true and wait for remote closing detach
        // The sender will be dropped after close
        if let Err(e) = self.link.send_detach(&mut self.outgoing, true, error).await {
            return Err(DetachError::new(false, Some(e)));
        }

        // Wait for remote detach
        let frame = match self.incoming.next().await {
            Some(frame) => frame,
            None => return Err(detach_error_expecting_frame()),
        };
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame()),
        };

        if remote_detach.closed {
            // If the remote detach contains an error, the error will be propagated
            // back by `on_incoming_detach`
            match self.link.on_incoming_detach(remote_detach).await {
                Ok(_) => {}
                Err(e) => {
                    return Err(DetachError {
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

            let session_control = self.session.clone();
            self.reattach_inner(session_control).await?;
            let frame = match self.incoming.next().await {
                Some(frame) => frame,
                None => return Err(detach_error_expecting_frame()),
            };

            // TODO: is checking closing still necessary?
            let _remote_detach = match frame {
                LinkFrame::Detach(detach) => detach,
                _ => return Err(detach_error_expecting_frame()),
            };
            match self.link.send_detach(&mut self.outgoing, true, None).await {
                Ok(_) => {}
                Err(e) => return Err(DetachError::new(false, Some(e))),
            }
        };

        Ok(())
    }

    pub(crate) async fn send<T>(&mut self, sendable: Sendable<T>) -> Result<Settlement, SendError>
    where
        T: serde::Serialize,
    {
        self.send_with_state(sendable, None).await
    }

    pub(crate) async fn send_with_state<T>(
        &mut self,
        sendable: Sendable<T>,
        state: Option<DeliveryState>,
    ) -> Result<Settlement, SendError>
    where
        T: serde::Serialize,
    {
        use bytes::BufMut;
        use serde::Serialize;
        use serde_amqp::ser::Serializer;

        let Sendable {
            message,
            message_format,
            settled,
        } = sendable;
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
            .send_payload(
                &mut self.outgoing,
                detached_fut,
                payload,
                message_format,
                settled,
                state,
                false,
            )
            .await?;
        Ok(settlement)
    }
}
