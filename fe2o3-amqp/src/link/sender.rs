//! Implementation of AMQP1.0 sender

use std::time::Duration;

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tokio::{
    sync::mpsc,
    time::{error::Elapsed, timeout},
};

use fe2o3_amqp_types::{
    definitions::{self, DeliveryTag, MessageFormat, SenderSettleMode},
    messaging::{
        message::__private::Serializable, Address, DeliveryState, Outcome, SerializableBody,
        Source, Target, MESSAGE_FORMAT,
    },
    performatives::{Attach, Detach, Transfer},
};
use tracing::instrument;

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
    ArcSenderUnsettledMap, LinkFrame, LinkRelay, LinkStateError, SendError, SenderAttachError,
    SenderAttachExchange, SenderFlowState, SenderLink, SenderResumeError, SenderResumeErrorKind,
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

    /// Get a reference to the link's source field
    pub fn source(&self) -> &Option<Source> {
        &self.inner.link.source
    }

    /// Get a reference to the link's target field
    pub fn target(&self) -> &Option<Target> {
        &self.inner.link.target
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
    pub async fn detach(mut self) -> Result<DetachedSender, DetachError> {
        self.inner.detach_with_error(None).await?;
        Ok(DetachedSender::new(self.inner))
    }

    /// Detach the link with an error
    pub async fn detach_with_error(
        mut self,
        error: impl Into<definitions::Error>,
    ) -> Result<DetachedSender, DetachError> {
        self.inner.detach_with_error(Some(error.into())).await?;
        Ok(DetachedSender::new(self.inner))
    }

    /// Detach the link with a timeout
    ///
    /// This simply wraps [`detach`](#method.detach) with a `timeout`
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
    pub async fn send<T: SerializableBody>(
        &mut self,
        sendable: impl Into<Sendable<T>>,
    ) -> Result<Outcome, SendError> {
        let fut = self.send_batchable(sendable).await?;
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
        let settlement = self.inner.send(sendable.into()).await?;

        Ok(DeliveryFut::from(settlement))
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
    pub(crate) async fn send<T>(&mut self, sendable: Sendable<T>) -> Result<Settlement, SendError>
    where
        T: SerializableBody,
    {
        self.send_with_state(sendable, None).await
    }

    pub(crate) async fn send_with_state<T, E>(
        &mut self,
        sendable: Sendable<T>,
        state: Option<DeliveryState>,
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

        self.send_payload(payload, message_format, settled, state)
            .await
    }

    pub(crate) async fn send_payload<E>(
        &mut self,
        payload: Payload,
        message_format: MessageFormat,
        settled: Option<bool>,
        state: Option<DeliveryState>,
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
                false,
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
    resend_buf: Vec<Payload>,
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
        Self {
            inner,
            resend_buf: Vec::new(),
        }
    }

    async fn handle_resuming_delivery(
        &mut self,
        delivery_tag: DeliveryTag,
        resuming: ResumingDelivery,
    ) -> Result<(), SendError> {
        tracing::debug!("Resuming delivery: delivery_tag: {:?}", delivery_tag);
        let settlement = match resuming {
            ResumingDelivery::Abort => self.inner.abort(delivery_tag).await?,
            ResumingDelivery::Resend(payload) => {
                self.resend_buf.push(payload);
                return Ok(());
            }
            ResumingDelivery::Resume { state, payload } => {
                self.inner.resume(delivery_tag, payload, state).await?
            }
            ResumingDelivery::RestateOutcome { local_state } => {
                self.inner
                    .restate_outcome(delivery_tag, local_state)
                    .await?
            }
        };

        let fut = DeliveryFut::<SendResult>::from(settlement);
        let outcome = fut.await?;
        tracing::debug!("Resuming delivery outcome {:?}", outcome);
        Ok(())
    }

    async fn resume_inner(
        &mut self,
        mut initial_remote_attach: Option<Attach>,
    ) -> Result<(), SenderResumeErrorKind> {
        self.inner.reallocate_output_handle().await?;

        loop {
            // let attach_exchange = self.inner.exchange_attach(false).await?;
            let attach_exchange = match initial_remote_attach.take() {
                Some(remote_attach) => {
                    self.inner
                        .link
                        .send_attach(&self.inner.outgoing, &self.inner.session, false)
                        .await?;
                    self.inner.link.on_incoming_attach(remote_attach)?
                }
                None => self.inner.exchange_attach(false).await?,
            };

            match attach_exchange {
                SenderAttachExchange::Complete => break,
                SenderAttachExchange::IncompleteUnsettled(resuming_deliveries) => {
                    for (delivery_tag, resuming) in resuming_deliveries {
                        self.handle_resuming_delivery(delivery_tag, resuming)
                            .await?;
                    }
                }
                SenderAttachExchange::Resume(resuming_deliveries) => {
                    for (delivery_tag, resuming) in resuming_deliveries {
                        self.handle_resuming_delivery(delivery_tag, resuming)
                            .await?;
                    }

                    // Resend buffered payloads
                    for payload in self.resend_buf.drain(..) {
                        let settlement = self
                            .inner
                            .send_payload::<SendError>(payload, MESSAGE_FORMAT, None, None)
                            .await?;
                        let fut = DeliveryFut::<SendResult>::from(settlement);

                        let outcome = fut.await?;
                        tracing::debug!("Resuming delivery outcome {:?}", outcome)
                    }

                    // Upon completion of this reduction of state, the two parties MUST suspend and
                    // re-attempt to resume the link.
                    self.inner.detach_with_error(None).await?;
                }
            }
        }

        Ok(())
    }

    // async fn on_attach_exchange(&mut self, attach_exchange: SenderAttachExchange) -> Result<(), SenderResumeErrorKind> {}

    /// Resume the sender link on the original session
    #[instrument(skip(self))]
    pub async fn resume(mut self) -> Result<Sender, SenderResumeError> {
        try_as_sender!(self, self.resume_inner(None).await);
        Ok(Sender { inner: self.inner })
    }

    /// Resume the sender link on the original session with an Attach sent by the remote peer
    pub async fn resume_incoming_attach(
        mut self,
        remote_attach: Attach,
    ) -> Result<Sender, SenderResumeError> {
        try_as_sender!(self, self.resume_inner(Some(remote_attach)).await);
        Ok(Sender { inner: self.inner })
    }

    /// Resume the sender link with a timeout
    #[instrument(skip(self))]
    pub async fn resume_with_timeout(
        mut self,
        duration: Duration,
    ) -> Result<Sender, SenderResumeError> {
        let fut = self.resume_inner(None);

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
        let fut = self.resume_inner(Some(remote_attach));

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
