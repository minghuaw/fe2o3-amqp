//! Implementation of AMQP1.0 sender

use std::time::Duration;

use async_trait::async_trait;
use bytes::BytesMut;
use tokio::{
    sync::mpsc,
    time::{error::Elapsed, timeout},
};

use fe2o3_amqp_types::{
    definitions::{self},
    messaging::{message::__private::Serializable, Address, DeliveryState, Target},
    performatives::Detach,
};

use crate::{
    control::SessionControl,
    endpoint::{self, LinkAttach, LinkDetach, LinkExt, Settlement},
    session::SessionHandle,
};

use super::{
    builder::{self, WithoutName, WithoutTarget, WithSource},
    delivery::{DeliveryFut, Sendable},
    error::DetachError,
    role,
    shared_inner::{LinkEndpointInner, LinkEndpointInnerDetach, recv_remote_detach},
    ArcSenderUnsettledMap, LinkFrame, LinkRelay, LinkStateError, SendError, SenderAttachError,
    SenderFlowState, SenderLink,
};

/// An AMQP1.0 sender
///
/// # Attach a new sender with default configurations
///
/// ```rust, ignore
/// let mut sender = Sender::attach(
///     &mut session,           // mutable reference to SessionHandle
///     "rust-sender-link-1",   // link name
///     "q1"                    // Target address
/// ).await.unwrap();
/// 
/// sender.send("hello AMQP").await.unwrap();
/// 
/// sender.close().await.unwrap();
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
    pub(crate) inner: SenderInner<SenderLink<Target>>,
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
    pub fn builder() -> builder::Builder<role::Sender, Target, WithoutName, WithSource, WithoutTarget> {
        builder::Builder::<role::Sender, Target, _, _, _>::new()
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
    ) -> Result<Sender, SenderAttachError> {
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
    pub async fn detach(mut self) -> Result<DetachedSender, DetachError> {
        self.inner.detach_with_error(None).await?;
        Ok(DetachedSender { _inner: self.inner })
    }

    /// Detach the link with an error
    pub async fn detach_with_error(
        mut self,
        error: definitions::Error,
    ) -> Result<DetachedSender, DetachError> {
        self.inner.detach_with_error(Some(error)).await?;
        Ok(DetachedSender { _inner: self.inner })
    }

    /// Detach the link with a timeout
    ///
    /// This simply wraps [`detach`] with a `timeout`
    pub async fn detach_with_timeout(
        self,
        duration: Duration,
    ) -> Result<Result<DetachedSender, DetachError>, Elapsed> {
        timeout(duration, self.detach()).await
    }

    /// Close the link.
    ///
    /// This will set the `closed` field in the Detach performative to true
    pub async fn close(mut self) -> Result<(), DetachError> {
        self.inner.close_with_error(None).await
    }

    /// Detach the link with an error
    pub async fn close_with_error(mut self, error: definitions::Error) -> Result<(), DetachError> {
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
        let fut = self.send_batchable(sendable).await?;
        fut.await
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

    /// Returns when the remote peer detach/close the link
    pub async fn on_detach(&mut self) -> DetachError {
        match recv_remote_detach(&mut self.inner).await {
            Ok(detach) => {
                let closed = detach.closed;
                match self.inner.link.on_incoming_detach(detach).await {
                    Ok(_) => if closed {
                        DetachError::ClosedByRemote
                    } else {
                        DetachError::DetachedByRemote
                    },
                    Err(err) => err,
                }
            },
            Err(err) => err,
        }
    }
}

/// A detached sender
///
/// # Link re-attachment
///
/// TODO
#[derive(Debug)]
pub struct DetachedSender {
    _inner: SenderInner<SenderLink<Target>>,
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
    pub(crate) outgoing: mpsc::Sender<LinkFrame>,
    pub(crate) incoming: mpsc::Receiver<LinkFrame>,
}

impl<L: endpoint::SenderLink> Drop for SenderInner<L> {
    fn drop(&mut self) {
        if let Some(handle) = self.link.output_handle_mut().take() {
            let detach = Detach {
                handle: handle.into(),
                closed: true,
                error: None,
            };
            let _ = self.outgoing.try_send(LinkFrame::Detach(detach));
        }
    }
}

#[async_trait]
impl<L> LinkEndpointInner for SenderInner<L>
where
    L: endpoint::SenderLink<AttachError = SenderAttachError, DetachError = DetachError>
        + LinkExt<FlowState = SenderFlowState, Unsettled = ArcSenderUnsettledMap>
        + Send
        + Sync,
{
    type Link = L;

    fn link(&self) -> &Self::Link {
        &self.link
    }

    fn link_mut(&mut self) -> &mut Self::Link {
        &mut self.link
    }

    fn writer(&self) -> &mpsc::Sender<LinkFrame> {
        &self.outgoing
    }

    fn reader_mut(&mut self) -> &mut mpsc::Receiver<LinkFrame> {
        &mut self.incoming
    }

    fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    fn as_new_link_relay(&self, tx: mpsc::Sender<LinkFrame>) -> LinkRelay<()> {
        LinkRelay::Sender {
            tx,
            output_handle: (),
            flow_state: self.link.flow_state().producer(),
            // TODO: what else to do during re-attaching
            unsettled: self.link.unsettled().clone(),
            receiver_settle_mode: self.link.rcv_settle_mode().clone(),
        }
    }

    fn session_control(&self) -> &mpsc::Sender<SessionControl> {
        &self.session
    }

    async fn negotiate_attach(&mut self) -> Result<(), <Self::Link as LinkAttach>::AttachError> {
        self.link
            .negotiate_attach(&self.outgoing, &mut self.incoming)
            .await
    }

    async fn handle_attach_error(
        &mut self,
        attach_error: <Self::Link as LinkAttach>::AttachError,
    ) -> <Self::Link as LinkAttach>::AttachError {
        self.link
            .handle_attach_error(
                attach_error,
                &self.outgoing,
                &mut self.incoming,
                &self.session,
            )
            .await
    }

    async fn send_detach(
        &mut self,
        closed: bool,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError> {
        self.link.send_detach(&self.outgoing, closed, error).await
    }
}

impl<L> SenderInner<L>
where
    L: endpoint::SenderLink<
            TransferError = LinkStateError,
            AttachError = SenderAttachError,
            DetachError = DetachError,
        > + LinkExt<FlowState = SenderFlowState, Unsettled = ArcSenderUnsettledMap>
        + Send
        + Sync,
{
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
        Serializable(message)
            .serialize(&mut serializer)
            .map_err(|_| SendError::MessageEncodeError)?;
        // let payload = BytesMut::from(payload);
        let payload = payload.freeze();

        // send a transfer, checking state will be implemented in SenderLink
        let detached_fut = self.incoming.recv();
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
