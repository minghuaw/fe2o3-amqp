//! Implementation of AMQP1.0 sender

use std::time::Duration;

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tokio::{
    sync::mpsc,
    time::{error::Elapsed, timeout},
};

use fe2o3_amqp_types::{
    definitions::{self, DeliveryTag, Fields, MessageFormat, SenderSettleMode},
    messaging::{
        message::__private::Serializable, Address, DeliveryState, Outcome, SerializableBody,
        Source, Target, MESSAGE_FORMAT,
    },
    performatives::{Attach, Detach, Transfer},
};

use crate::{
    control::SessionControl,
    endpoint::{self, LinkAttach, LinkDetach, LinkExt, Settlement},
    session::SessionHandle,
    Payload,
};

use super::{
    builder::{self, WithSource, WithoutName, WithoutTarget},
    delivery::{DeliveryFut, SendResult, Sendable},
    error::DetachError,
    resumption::ResumingDelivery,
    role,
    shared_inner::{
        recv_remote_detach, LinkEndpointInner, LinkEndpointInnerDetach, LinkEndpointInnerReattach,
    },
    ArcSenderUnsettledMap, DetachThenResumeSenderError, LinkFrame, LinkRelay, LinkStateError,
    SendError, SenderAttachError, SenderAttachExchange, SenderFlowState, SenderLink,
    SenderResumeError, SenderResumeErrorKind,
};

#[cfg(docsrs)]
use fe2o3_amqp_types::messaging::{AmqpSequence, AmqpValue, Batch, Body, Data, IntoBody, Message};

/// An AMQP1.0 sender
///
/// # Attach a new sender with default configurations
///
/// ```rust
/// let mut sender = Sender::attach(
///     &mut session,           // mutable reference to SessionHandle
///     "rust-sender-link-1",   // link name
///     "q1"                    // Target address
/// ).await.unwrap();
///
/// let outcome = sender.send("hello AMQP").await.unwrap();
///
/// // Checks the outcome of delivery
/// if outcome.is_accepted() {
///     tracing::info!("Outcome: {:?}", outcome)
/// } else {
///     tracing::error!("Outcome: {:?}", outcome)
/// }
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
/// ```rust
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
        f.debug_struct("Sender").finish()
    }
}

impl Sender {
    /// Creates a builder for [`Sender`] link
    pub fn builder(
    ) -> builder::Builder<role::SenderMarker, Target, WithoutName, WithSource, WithoutTarget> {
        builder::Builder::<role::SenderMarker, Target, _, _, _>::new()
    }

    /// Get the name of the link
    pub fn name(&self) -> &str {
        self.inner.link.name()
    }

    /// Returns the `max_message_size` of the link. A value of zero indicates that the link has no
    /// maximum message size, and thus a zero value is turned into a `None`
    pub fn max_message_size(&self) -> Option<u64> {
        self.inner.link.max_message_size()
    }

    /// Get a reference to the link's source field
    pub fn source(&self) -> &Option<Source> {
        &self.inner.link.source
    }

    /// Get a reference to the link's target field
    pub fn target(&self) -> &Option<Target> {
        &self.inner.link.target
    }

    /// Get a reference to the link's properties field in the op
    pub fn properties<F, O>(&self, op: F) -> O
    where
        F: FnOnce(&Option<Fields>) -> O,
    {
        self.inner.link.properties(op)
    }

    /// Get a mutable reference to the link's properties field in the op
    pub fn properties_mut<F, O>(&mut self, op: F) -> O
    where
        F: FnOnce(&mut Option<Fields>) -> O,
    {
        self.inner.link.properties_mut(op)
    }

    /// Attach the sender link to a session with default configuration
    /// with the `name` and `target` address set to the specified values
    ///
    /// ## Default configuration
    ///
    /// | Field | Default Value |
    /// |-------|---------------|
    /// |`name`|`String::default()`|
    /// |`snd_settle_mode`|`[SenderSettleMode]::Mixed`|
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
    /// ```rust
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
    pub async fn detach(mut self) -> Result<DetachedSender, (DetachedSender, DetachError)> {
        match self.inner.detach_with_error(None).await {
            Ok(_) => Ok(DetachedSender::new(self.inner)),
            Err(err) => Err((DetachedSender::new(self.inner), err)),
        }
    }

    /// Detach the link with an error
    pub async fn detach_with_error(
        mut self,
        error: impl Into<definitions::Error>,
    ) -> Result<DetachedSender, (DetachedSender, DetachError)> {
        match self.inner.detach_with_error(Some(error.into())).await {
            Ok(_) => Ok(DetachedSender::new(self.inner)),
            Err(err) => Err((DetachedSender::new(self.inner), err)),
        }
    }

    /// Detach the link with a timeout
    ///
    /// This simply wraps [`detach`](#method.detach) with a `timeout`
    pub async fn detach_with_timeout(
        self,
        duration: Duration,
    ) -> Result<Result<DetachedSender, (DetachedSender, DetachError)>, Elapsed> {
        timeout(duration, self.detach()).await
    }

    /// Detach and re-attach the link to a new session
    pub async fn detach_then_resume_on_session<R>(
        &mut self,
        new_session: &SessionHandle<R>,
    ) -> Result<(), DetachThenResumeSenderError> {
        // Detach the link
        self.inner.detach_with_error(None).await?;

        // Re-attach the link
        *self.inner.session_control_mut() = new_session.control.clone();
        self.inner.resume_incoming_attach(None).await?;
        Ok(())
    }

    /// Close the link.
    ///
    /// This will set the `closed` field in the Detach performative to true
    pub async fn close(mut self) -> Result<(), DetachError> {
        self.inner.close_with_error(None).await
    }

    /// Detach the link with an error
    pub async fn close_with_error(
        mut self,
        error: impl Into<definitions::Error>,
    ) -> Result<(), DetachError> {
        self.inner.close_with_error(Some(error.into())).await
    }

    /// Send a message and wait for acknowledgement (disposition)
    ///
    /// # Use custom types as argument
    ///
    /// The AMQP 1.0 protocol requires user to choose the body section type:
    ///
    /// - [`AmqpValue`]
    /// - [`AmqpSequence`] / [`Batch<AmqpSequence>`]
    /// - [`Data`] / [`Batch<Data>`]
    ///
    /// Below shows some ways to use custom types.
    ///
    /// ## 1. Wrap custom type in [`AmqpValue`]
    ///
    /// The easiest way to use a custom type is probably to define a custom type that implements
    /// [`serde::Serialize`] and then wrap the type in [`AmqpValue`].
    ///
    /// ### Example
    ///
    /// ```rust
    /// #[derive(Serialize)]
    /// struct Foo {
    ///     a: i32
    /// }
    ///
    /// sender.send(AmqpValue(Foo{a: 3})).await.unwrap();
    /// ```
    ///
    /// ## 2. Implement [`IntoBody`] trait for the custom type
    ///
    /// Another option is to implement the [`IntoBody`] trait, which essentially asks the user to
    /// choose which body section type should be used. Please see documention on [`IntoBody`] for
    /// more information on the available body section types.
    ///
    /// ### Example
    ///
    /// ```rust
    /// #[derive(Serialize)]
    /// struct Foo {
    ///     a: i32
    /// }
    ///
    /// impl IntoBody for Foo {
    ///     type Body = AmqpValue<Foo>;
    ///
    ///     fn into_body(self) -> Self::Body {
    ///         AmqpValue(self)
    ///     }
    /// }
    ///
    /// sender.send(Foo{a: 5}).await.unwrap();
    /// ```
    ///
    /// ## 3. Use message builder
    ///
    /// One could still use the message builder to not only set other non-body fields of
    /// [`Message`] but also choosing the type of body section.
    ///
    /// ```rust
    /// #[derive(Serialize)]
    /// struct Foo {
    ///     a: i32
    /// }
    ///
    /// let message = Message::builder()
    ///     .properties(
    ///         Properties::builder()
    ///         .group_id(String::from("send_to_event_hub"))
    ///         .build(),
    ///     )
    ///     .value(Foo {a: 8}) // set body section to `AmqpValue<Foo>`
    ///     .build();
    /// sender.send(message).await.unwrap();
    /// ```
    ///
    /// # Pre-settle a message with [`Sendable`] builder
    ///
    /// The user can pre-settle a message by specifying `settled` field in the [`Sendable`]. Unless
    /// an explicity [`Sendable`] is passed to the argument, the `message_format` field is set to
    /// [`MESSAGE_FORMAT`] and the `settled` field is set to `None`.
    ///
    /// ## Example
    ///
    /// Send message pre-settled if the sender's settle mode is set to `SenderSettleMode::Mixed`.
    /// Please note that the field `settled` will be ***ignored*** if the negotiated
    /// `SenderSettleMode` is set to either `SenderSettleMode::Settled` or
    /// `SenderSettleMode::Unsettled`
    ///
    /// ```rust
    /// let sendable = Sendable::builder()
    ///     .message("hello AMQP")
    ///     .settled(true)
    ///     .build();
    /// let outcome = sender.send(sendable).await.unwrap():
    /// ```
    ///
    /// # Cancel safety
    ///
    /// This function is cancel-safe. See [#22](https://github.com/minghuaw/fe2o3-amqp/issues/22)
    /// for more details.
    pub async fn send<T: SerializableBody>(
        &mut self,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<Outcome, SendError> {
        let fut = self
            .inner
            .send_with_state::<T, SendError>(sendable.into(), None, false)
            .await
            .map(DeliveryFut::from)?;
        fut.await
    }

    /// Like [`send()`](#method.send) but takes a reference to the message
    ///
    /// This is useful when the message is large and you want to avoid cloning it because the
    /// message may be used again after the send operation.
    pub async fn send_ref<T: SerializableBody>(
        &mut self,
        sendable: &Sendable<T>,
    ) -> Result<Outcome, SendError> {
        let fut = self
            .inner
            .send_ref_with_state::<T, SendError>(sendable, None, false)
            .await
            .map(DeliveryFut::from)?;
        fut.await
    }

    /// Send a message and wait for acknowledgement (disposition) with a timeout.
    ///
    /// This simply wraps [`send`](#method.send) inside a [`tokio::time::timeout`]
    pub async fn send_with_timeout<T: SerializableBody>(
        &mut self,
        sendable: impl Into<Sendable<T>>,
        duration: Duration,
    ) -> Result<Result<Outcome, SendError>, Elapsed> {
        timeout(duration, self.send(sendable)).await
    }

    /// Send a message without waiting for the acknowledgement.
    ///
    /// This will set the batchable field of the `Transfer` performative to true. Please see
    /// [`send()`](#method.send) for information on how to use custom type as argument.
    ///
    /// # Example
    ///
    /// ```rust
    /// let fut = sender.send_batchable("HELLO AMQP").await.unwrap();
    /// let result = fut.await;
    /// println!("fut {:?}", result);
    /// ```
    pub async fn send_batchable<T: SerializableBody>(
        &mut self,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<DeliveryFut<Result<Outcome, SendError>>, SendError> {
        self.inner
            .send_with_state(sendable.into(), None, true)
            .await
            .map(DeliveryFut::from)
    }

    /// Like [`send_batchable()`](#method.send_batchable) but this only takes a reference.
    ///
    /// This is useful when the message is large and you want to avoid cloning it because the
    /// message may be used again after the send operation.
    pub async fn send_batchable_ref<T: SerializableBody>(
        &mut self,
        sendable: &Sendable<T>,
    ) -> Result<DeliveryFut<Result<Outcome, SendError>>, SendError> {
        self.inner
            .send_ref_with_state(sendable, None, true)
            .await
            .map(DeliveryFut::from)
    }

    /// Returns when the remote peer detach/close the link
    pub async fn on_detach(&mut self) -> DetachError {
        match recv_remote_detach(&mut self.inner).await {
            Ok(detach) => {
                let closed = detach.closed;
                match self.inner.link.on_incoming_detach(detach) {
                    Ok(_) => {
                        if closed {
                            DetachError::ClosedByRemote
                        } else {
                            DetachError::DetachedByRemote
                        }
                    }
                    Err(err) => err,
                }
            }
            Err(err) => err,
        }
    }
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
        + LinkAttach<AttachExchange = SenderAttachExchange>
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

    fn session_control_mut(&mut self) -> &mut mpsc::Sender<SessionControl> {
        &mut self.session
    }

    async fn exchange_attach(
        &mut self,
        is_reattaching: bool,
    ) -> Result<SenderAttachExchange, <Self::Link as LinkAttach>::AttachError> {
        self.link
            .exchange_attach(
                &self.outgoing,
                &mut self.incoming,
                &self.session,
                is_reattaching,
            )
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

#[async_trait]
impl<L> LinkEndpointInnerReattach for SenderInner<L>
where
    L: endpoint::SenderLink<AttachError = SenderAttachError, DetachError = DetachError>
        + LinkExt<FlowState = SenderFlowState, Unsettled = ArcSenderUnsettledMap>
        + LinkAttach<AttachExchange = SenderAttachExchange>
        + Send
        + Sync,
{
    fn handle_reattach_outcome(
        &mut self,
        outcome: SenderAttachExchange,
    ) -> Result<&mut Self, L::AttachError> {
        match outcome {
            SenderAttachExchange::Complete => Ok(self),
            //  Re-attach should have None valued unsettled, so this should be invalid
            SenderAttachExchange::IncompleteUnsettled(_) | SenderAttachExchange::Resume(_) => {
                Err(SenderAttachError::IllegalState)
            }
        }
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
    pub(crate) async fn send_with_state<T, E>(
        &mut self,
        sendable: Sendable<T>,
        state: Option<DeliveryState>,
        batchable: bool,
    ) -> Result<Settlement, E>
    where
        T: SerializableBody,
        E: From<L::TransferError> + From<serde_amqp::Error>,
    {
        use bytes::BufMut;
        use serde::Serialize;
        use serde_amqp::ser::Serializer;

        let Sendable {
            message,
            message_format,
            settled,
        } = sendable;

        // serialize message
        let mut payload = BytesMut::new();
        let mut serializer = Serializer::from((&mut payload).writer());
        Serializable(message).serialize(&mut serializer)?;
        let payload = payload.freeze();

        self.send_payload(payload, message_format, settled, state, batchable)
            .await
    }

    pub(crate) async fn send_ref_with_state<T, E>(
        &mut self,
        sendable: &Sendable<T>,
        state: Option<DeliveryState>,
        batchable: bool,
    ) -> Result<Settlement, E>
    where
        T: SerializableBody,
        E: From<L::TransferError> + From<serde_amqp::Error>,
    {
        use bytes::BufMut;
        use serde::Serialize;
        use serde_amqp::ser::Serializer;

        let Sendable {
            message,
            message_format,
            settled,
        } = sendable;

        // serialize message
        let mut payload = BytesMut::new();
        let mut serializer = Serializer::from((&mut payload).writer());
        Serializable(message).serialize(&mut serializer)?;
        let payload = payload.freeze();

        self.send_payload(payload, *message_format, *settled, state, batchable)
            .await
    }

    pub(crate) async fn send_payload<E>(
        &mut self,
        payload: Payload,
        message_format: MessageFormat,
        settled: Option<bool>,
        state: Option<DeliveryState>,
        batchable: bool,
    ) -> Result<Settlement, E>
    where
        E: From<L::TransferError> + From<serde_amqp::Error>,
    {
        // send a transfer, checking state will be implemented in SenderLink
        let detached_fut = self.incoming.recv(); // cancel safe
        let settlement = self
            .link
            .send_payload(
                &self.outgoing,
                detached_fut,
                payload,
                message_format,
                settled,
                state,
                batchable,
            )
            .await?;
        Ok(settlement)
    }
}

impl SenderInner<SenderLink<Target>> {
    async fn abort(&mut self, delivery_tag: DeliveryTag) -> Result<Settlement, SendError> {
        let handle = self
            .link
            .output_handle
            .clone()
            .ok_or(LinkStateError::IllegalState)?
            .into();
        let transfer = Transfer {
            handle,
            delivery_id: None,
            delivery_tag: Some(delivery_tag),
            message_format: Some(MESSAGE_FORMAT),
            settled: None,
            more: false,
            rcv_settle_mode: None,
            state: None,
            resume: true,
            aborted: true,
            batchable: false,
        };
        let payload = Bytes::new();

        endpoint::SenderLink::send_payload_with_transfer(
            &mut self.link,
            &self.outgoing,
            transfer,
            payload,
        )
        .await
        .map_err(Into::into)
    }

    async fn handle_resuming_delivery(
        &mut self,
        delivery_tag: DeliveryTag,
        resuming: ResumingDelivery,
        resend_buf: &mut Vec<Payload>,
    ) -> Result<(), SendError> {
        #[cfg(feature = "tracing")]
        tracing::debug!("Resuming delivery: delivery_tag: {:?}", delivery_tag);
        #[cfg(feature = "log")]
        log::debug!("Resuming delivery: delivery_tag: {:?}", delivery_tag);
        let settlement = match resuming {
            ResumingDelivery::Abort => self.abort(delivery_tag).await?,
            ResumingDelivery::Resend(payload) => {
                resend_buf.push(payload);
                return Ok(());
            }
            ResumingDelivery::Resume { state, payload } => {
                self.resume(delivery_tag, payload, state).await?
            }
            ResumingDelivery::RestateOutcome { local_state } => {
                self.restate_outcome(delivery_tag, local_state).await?
            }
        };

        let fut = DeliveryFut::<SendResult>::from(settlement);
        let _outcome = fut.await?;
        #[cfg(feature = "tracing")]
        tracing::debug!("Resuming delivery outcome {:?}", _outcome);
        #[cfg(feature = "log")]
        log::debug!("Resuming delivery outcome {:?}", _outcome);
        Ok(())
    }

    async fn resume(
        &mut self,
        delivery_tag: DeliveryTag,
        payload: Payload,
        state: Option<DeliveryState>,
    ) -> Result<Settlement, SendError> {
        let handle = self
            .link
            .output_handle
            .clone()
            .ok_or(LinkStateError::IllegalState)?
            .into();
        let settled = match self.link.snd_settle_mode {
            SenderSettleMode::Settled => true,
            SenderSettleMode::Unsettled => false,
            SenderSettleMode::Mixed => false,
        };
        let transfer = Transfer {
            handle,
            delivery_id: None,
            delivery_tag: Some(delivery_tag),
            message_format: Some(MESSAGE_FORMAT),
            settled: Some(settled),
            more: false, // This will be determined in `send_payload_with_transfer`
            rcv_settle_mode: None,
            state,
            resume: true,
            aborted: false,
            batchable: false,
        };

        endpoint::SenderLink::send_payload_with_transfer(
            &mut self.link,
            &self.outgoing,
            transfer,
            payload,
        )
        .await
        .map_err(Into::into)
    }

    async fn restate_outcome(
        &mut self,
        delivery_tag: DeliveryTag,
        state: DeliveryState,
    ) -> Result<Settlement, SendError> {
        let handle = self
            .link
            .output_handle
            .clone()
            .ok_or(LinkStateError::IllegalState)?
            .into();
        let transfer = Transfer {
            handle,
            delivery_id: None,
            delivery_tag: Some(delivery_tag),
            message_format: Some(MESSAGE_FORMAT),
            settled: Some(false),
            more: false,
            rcv_settle_mode: None,
            state: Some(state),
            resume: true,
            aborted: false,
            batchable: false,
        };
        let payload = Bytes::new();

        endpoint::SenderLink::send_payload_with_transfer(
            &mut self.link,
            &self.outgoing,
            transfer,
            payload,
        )
        .await
        .map_err(Into::into)
    }

    async fn resume_incoming_attach(
        &mut self,
        mut initial_remote_attach: Option<Attach>,
    ) -> Result<(), SenderResumeErrorKind> {
        self.reallocate_output_handle().await?;
        let mut resend_buf = Vec::new();

        loop {
            let attach_exchange = match initial_remote_attach.take() {
                Some(remote_attach) => {
                    self.link
                        .send_attach(&self.outgoing, &self.session, false)
                        .await?;
                    self.link.on_incoming_attach(remote_attach)?
                }
                None => self.exchange_attach(false).await?,
            };

            match attach_exchange {
                SenderAttachExchange::Complete => break,
                SenderAttachExchange::IncompleteUnsettled(resuming_deliveries) => {
                    for (delivery_tag, resuming) in resuming_deliveries {
                        self.handle_resuming_delivery(delivery_tag, resuming, &mut resend_buf)
                            .await?;
                    }
                }
                SenderAttachExchange::Resume(resuming_deliveries) => {
                    for (delivery_tag, resuming) in resuming_deliveries {
                        self.handle_resuming_delivery(delivery_tag, resuming, &mut resend_buf)
                            .await?;
                    }

                    // Resend buffered payloads
                    for payload in resend_buf.drain(..) {
                        let settlement = self
                            .send_payload::<SendError>(payload, MESSAGE_FORMAT, None, None, false)
                            .await?;
                        let fut = DeliveryFut::<SendResult>::from(settlement);

                        let _outcome = fut.await?;
                        #[cfg(feature = "tracing")]
                        tracing::debug!("Resuming delivery outcome {:?}", _outcome);
                        #[cfg(feature = "log")]
                        log::debug!("Resuming delivery outcome {:?}", _outcome);
                    }

                    // Upon completion of this reduction of state, the two parties MUST suspend and
                    // re-attempt to resume the link.
                    self.detach_with_error(None).await?;
                }
            }
        }

        Ok(())
    }
}

/// A detached sender
///
/// # Example
///
/// Link re-attachment
///
/// ```rust
/// let detached = sender.detach().await.unwrap();
/// let sender = detached.resume().await.unwrap():
/// ```
#[derive(Debug)]
pub struct DetachedSender {
    inner: SenderInner<SenderLink<Target>>,
}

macro_rules! try_as_sender {
    ($self:ident, $f:expr) => {
        match $f {
            Ok(outcome) => outcome,
            Err(error) => {
                return Err(SenderResumeError {
                    detached_sender: $self,
                    kind: error.into(),
                })
            }
        }
    };
}

impl DetachedSender {
    fn new(inner: SenderInner<SenderLink<Target>>) -> Self {
        Self { inner }
    }

    /// Resume the sender link on the original session
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub async fn resume(mut self) -> Result<Sender, SenderResumeError> {
        try_as_sender!(self, self.inner.resume_incoming_attach(None).await);
        Ok(Sender { inner: self.inner })
    }

    /// Resume the sender link on the original session with an Attach sent by the remote peer
    pub async fn resume_incoming_attach(
        mut self,
        remote_attach: Attach,
    ) -> Result<Sender, SenderResumeError> {
        try_as_sender!(
            self,
            self.inner.resume_incoming_attach(Some(remote_attach)).await
        );
        Ok(Sender { inner: self.inner })
    }

    /// Resume the sender link with a timeout
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub async fn resume_with_timeout(
        mut self,
        duration: Duration,
    ) -> Result<Sender, SenderResumeError> {
        let fut = self.inner.resume_incoming_attach(None);

        match tokio::time::timeout(duration, fut).await {
            Ok(Ok(_)) => Ok(Sender { inner: self.inner }),
            Ok(Err(kind)) => Err(SenderResumeError {
                detached_sender: self,
                kind,
            }),
            Err(_) => {
                try_as_sender!(self, self.inner.detach_with_error(None).await);
                Err(SenderResumeError {
                    detached_sender: self,
                    kind: SenderResumeErrorKind::Timeout,
                })
            }
        }
    }

    /// Resume the sender link on the original session with an Attach sent by the remote peer
    pub async fn resume_incoming_attach_with_timeout(
        mut self,
        remote_attach: Attach,
        duration: Duration,
    ) -> Result<Sender, SenderResumeError> {
        let fut = self.inner.resume_incoming_attach(Some(remote_attach));

        match tokio::time::timeout(duration, fut).await {
            Ok(Ok(_)) => Ok(Sender { inner: self.inner }),
            Ok(Err(kind)) => Err(SenderResumeError {
                detached_sender: self,
                kind,
            }),
            Err(_) => {
                try_as_sender!(self, self.inner.detach_with_error(None).await);
                Err(SenderResumeError {
                    detached_sender: self,
                    kind: SenderResumeErrorKind::Timeout,
                })
            }
        }
    }

    /// Resume the sender on a specific session
    pub async fn resume_on_session<R>(
        mut self,
        session: &SessionHandle<R>,
    ) -> Result<Sender, SenderResumeError> {
        *self.inner.session_control_mut() = session.control.clone();
        self.resume().await
    }

    /// Resume the sender on a specific session
    pub async fn resume_incoming_attach_on_session<R>(
        mut self,
        remote_attach: Attach,
        session: &SessionHandle<R>,
    ) -> Result<Sender, SenderResumeError> {
        *self.inner.session_control_mut() = session.control.clone();
        self.resume_incoming_attach(remote_attach).await
    }

    /// Resume the sender on a specific session with timeout
    pub async fn resume_on_session_with_timeout<R>(
        mut self,
        session: &SessionHandle<R>,
        duration: Duration,
    ) -> Result<Sender, SenderResumeError> {
        *self.inner.session_control_mut() = session.control.clone();
        self.resume_with_timeout(duration).await
    }

    /// Resume the sender on a specific session with timeout
    pub async fn resume_incoming_attach_on_session_with_timeout<R>(
        mut self,
        remote_attach: Attach,
        session: &SessionHandle<R>,
        duration: Duration,
    ) -> Result<Sender, SenderResumeError> {
        *self.inner.session_control_mut() = session.control.clone();
        self.resume_incoming_attach_with_timeout(remote_attach, duration)
            .await
    }
}
