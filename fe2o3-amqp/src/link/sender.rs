//! Implementation of AMQP1.0 sender

use bytes::{Bytes, BytesMut};
use tokio::sync::{mpsc, oneshot};

cfg_not_wasm32! {
    use std::time::Duration;
    use tokio::time::{error::Elapsed, timeout};
}

use fe2o3_amqp_types::{
    definitions::{self, DeliveryTag, Fields, MessageFormat, SenderSettleMode},
    messaging::{
        message::__private::Serializable, Address, DeliveryState, Outcome, SerializableBody,
        Source, Target,
    },
    performatives::{Attach, Detach, Transfer},
    primitives::OrderedMap,
};

use crate::{
    control::SessionControl,
    endpoint::{self, LinkAttach, LinkDetach, LinkExt, Settlement},
    session::SessionHandle,
    Payload,
};

use super::{
    builder::{self, WithSource, WithoutName, WithoutTarget},
    delivery::{DeliveryFut, Sendable, UnsettledMessage},
    error::DetachError,
    resumption::ResumingDelivery,
    role,
    shared_inner::{
        recv_remote_detach, LinkEndpointInner, LinkEndpointInnerDetach, LinkEndpointInnerReattach,
    },
    ArcSenderUnsettledMap, AttachMode, DetachThenResumeSenderError, LinkFrame, LinkRelay,
    LinkStateError, SendError, SenderAttachError, SenderAttachExchange, SenderFlowState,
    SenderLink, SenderResumeError, SenderResumeErrorKind,
};

#[cfg(docsrs)]
use fe2o3_amqp_types::messaging::{
    AmqpSequence, AmqpValue, Batch, Body, Data, IntoBody, Message, MESSAGE_FORMAT,
};

/// An AMQP1.0 sender
///
/// # Attach a new sender with default configurations
///
/// ```rust,ignore
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
/// ```rust,ignore
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

    /// Get a mutable reference to the link's source field
    pub fn source_mut(&mut self) -> &mut Option<Source> {
        &mut self.inner.link.source
    }

    /// Get a reference to the link's target field
    pub fn target(&self) -> &Option<Target> {
        &self.inner.link.target
    }

    /// Get a mutable reference to the link's target field
    pub fn target_mut(&mut self) -> &mut Option<Target> {
        &mut self.inner.link.target
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
    /// ```rust,ignore
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

    cfg_not_wasm32! {
        /// Detach the link with a timeout
        ///
        /// This simply wraps [`detach`](#method.detach) with a `timeout`
        pub async fn detach_with_timeout(
            self,
            duration: Duration,
        ) -> Result<Result<DetachedSender, (DetachedSender, DetachError)>, Elapsed> {
            timeout(duration, self.detach()).await
        }
    }

    /// Detach and re-attach the link to a new session
    ///
    /// This will still attempt to re-attach even if detaching fails.
    /// `DetachThenResumeSenderError::Resume` will be returned if the detach succeeds but re-attach
    /// fails. `DetachThenResumeSenderError::Detach` will be returned if both the detach and
    /// re-attach fails.
    pub async fn detach_then_resume_on_session<R>(
        &mut self,
        new_session: &SessionHandle<R>,
    ) -> Result<(), DetachThenResumeSenderError> {
        // Detach the link
        let detach_result = self.inner.detach_with_error(None).await;

        // If the peer closed the link instead of suspending it, the link can
        // no longer be resumed. Return the existing detach error.
        if matches!(
            &detach_result,
            Err(DetachError::ClosedByRemote | DetachError::RemoteClosedWithError(_))
        ) {
            return Err(DetachThenResumeSenderError::Detach(
                detach_result.unwrap_err(),
            ));
        }

        // Re-attach the link
        self.inner.session = new_session.control.clone();
        self.inner.outgoing = new_session.outgoing.clone();
        let attach_result = self.inner.resume_incoming_attach(None).await;

        match (detach_result, attach_result) {
            (_, Ok(())) => Ok(()),
            (Ok(()), Err(e)) => Err(DetachThenResumeSenderError::Resume(e)),
            (Err(e), Err(_)) => Err(DetachThenResumeSenderError::Detach(e)),
        }
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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

    cfg_not_wasm32! {
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
    }

    /// Send a message without waiting for the acknowledgement.
    ///
    /// This will set the batchable field of the `Transfer` performative to true. Please see
    /// [`send()`](#method.send) for information on how to use custom type as argument.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let fut = sender.send_batchable("HELLO AMQP").await.unwrap();
    /// let result = fut.await;
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

    /// Source-only deliveries (the peer's attach map lacked their tags) that
    /// were removed from the advertised unsettled map during resumption and
    /// have to be re-sent as new, non-resumed deliveries once the exchange
    /// completes. Kept on the endpoint so a failed or cancelled resume can
    /// retry them.
    pub(crate) pending_redeliveries: Vec<UnsettledMessage>,
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

        // The buffered redeliveries were removed from the unsettled map on
        // resume; since this sender is gone they can no longer be re-sent.
        // Dropping them closes their settlement channels instead of letting
        // the delivery futures hang.
        self.pending_redeliveries.clear();
    }
}

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

    async fn exchange_attach(
        &mut self,
        mode: AttachMode,
    ) -> Result<SenderAttachExchange, <Self::Link as LinkAttach>::AttachError> {
        self.link
            .exchange_attach(&self.outgoing, &mut self.incoming, &self.session, mode)
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

/// Fail the deliveries carried by a map-carrying attach exchange without
/// resuming them: the reattach is completing a close (AMQP 1.0 §2.6.6), not
/// resuming the link, so the outstanding deliveries are dropped and their
/// delivery futures resolve with a link-state error instead of hanging.
fn fail_resuming_deliveries(resuming_deliveries: Vec<(DeliveryTag, ResumingDelivery)>) {
    for (_, resuming) in resuming_deliveries {
        match resuming {
            ResumingDelivery::Abort { sender, .. } => drop(sender),
            ResumingDelivery::Resend(message) | ResumingDelivery::Resume(message) => drop(message),
            ResumingDelivery::RestateOutcome { sender, .. } => drop(sender),
        }
    }
}

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
            SenderAttachExchange::Complete => {}
            // The reachable map-carrying exchanges are the peer's reply during
            // the crossed close: drop the deliveries it still considers
            // unsettled and let `reattach_and_then_close` send the closing
            // detach.
            SenderAttachExchange::IncompleteUnsettled(resuming_deliveries)
            | SenderAttachExchange::Resume(resuming_deliveries) => {
                fail_resuming_deliveries(resuming_deliveries);
            }
        }
        Ok(self)
    }
}

/// Maximum number of incomplete-unsettled suspend/re-attempt rounds before
/// resuming a sender link is aborted.
///
/// AMQP 1.0 §2.6.13 requires the two endpoints to reduce the unsettled state
/// and then suspend and re-attempt the resume. Each round should make
/// progress; this bounds the loop when it does not (e.g. the peer keeps
/// advertising the same unsettled map and the application never settles).
const MAX_INCOMPLETE_UNSETTLED_ROUNDS: usize = 32;

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
    /// Resumes a delivery with the given state and payload.
    ///
    /// The resume operation should not replace the unsettled map entry.
    async fn handle_resuming_delivery(
        &mut self,
        delivery_tag: DeliveryTag,
        resuming: ResumingDelivery,
    ) -> Result<(), SendError> {
        #[cfg(feature = "tracing")]
        tracing::debug!("Resuming delivery: delivery_tag: {:?}", delivery_tag);
        #[cfg(feature = "log")]
        log::debug!("Resuming delivery: delivery_tag: {:?}", delivery_tag);
        match resuming {
            ResumingDelivery::Abort {
                message_format,
                sender,
            } => self.abort(delivery_tag, message_format, sender).await?,
            ResumingDelivery::Resend(unsettled_message) => {
                self.pending_redeliveries.push(unsettled_message)
            }
            ResumingDelivery::Resume(unsettled_message) => {
                self.resume(delivery_tag, unsettled_message).await?
            }
            ResumingDelivery::RestateOutcome {
                message_format,
                local_state,
                payload,
                sender,
            } => {
                self.restate_outcome(delivery_tag, message_format, local_state, payload, sender)
                    .await?
            }
        }

        Ok(())
    }

    async fn abort(
        &mut self,
        delivery_tag: DeliveryTag,
        message_format: MessageFormat,
        sender: Option<oneshot::Sender<Option<DeliveryState>>>,
    ) -> Result<(), SendError> {
        let handle = self
            .link
            .output_handle
            .clone()
            .ok_or(LinkStateError::IllegalState)?
            .into();
        let transfer = Transfer {
            handle,
            delivery_id: None,
            delivery_tag: Some(delivery_tag.clone()),
            message_format: Some(message_format),
            settled: None,
            more: false,
            rcv_settle_mode: None,
            state: None,
            resume: true,
            aborted: true,
            batchable: false,
        };
        let payload = Bytes::new();

        let settled = self
            .link
            .send_transfer_without_modifying_unsettled_map(
                &self.outgoing,
                transfer,
                payload.clone(),
            )
            .await?;

        match settled {
            true => {
                if let Some(sender) = sender {
                    let _ = sender.send(None);
                }
            }
            false => {
                if let Some(sender) = sender {
                    let unsettled = UnsettledMessage::new(payload, None, message_format, sender);
                    let mut guard = self.link.unsettled.write();
                    guard
                        .get_or_insert(OrderedMap::new())
                        .insert(delivery_tag, unsettled);
                }
            }
        }

        Ok(())
    }

    async fn resume(
        &mut self,
        delivery_tag: DeliveryTag,
        unsettled_message: UnsettledMessage,
    ) -> Result<(), SendError> {
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
            delivery_tag: Some(delivery_tag.clone()),
            message_format: Some(unsettled_message.message_format),
            settled: Some(settled),
            more: false, // This will be determined in `send_payload_with_transfer`
            rcv_settle_mode: None,
            state: unsettled_message.state.clone(),
            resume: true,
            aborted: false,
            batchable: false,
        };

        let settled = self
            .link
            .send_transfer_without_modifying_unsettled_map(
                &self.outgoing,
                transfer,
                unsettled_message.payload.clone(),
            )
            .await?;

        match settled {
            true => {
                let _ = unsettled_message.settle();
            }
            false => {
                let mut guard = self.link.unsettled.write();
                guard
                    .get_or_insert(OrderedMap::new())
                    .insert(delivery_tag, unsettled_message);
            }
        }
        Ok(())
    }

    async fn restate_outcome(
        &mut self,
        delivery_tag: DeliveryTag,
        message_format: MessageFormat,
        state: DeliveryState,
        payload: Payload,
        sender: oneshot::Sender<Option<DeliveryState>>,
    ) -> Result<(), SendError> {
        let handle = self
            .link
            .output_handle
            .clone()
            .ok_or(LinkStateError::IllegalState)?
            .into();
        let transfer = Transfer {
            handle,
            delivery_id: None,
            delivery_tag: Some(delivery_tag.clone()),
            message_format: Some(message_format),
            settled: Some(false),
            more: false,
            rcv_settle_mode: None,
            state: Some(state),
            resume: true,
            aborted: false,
            batchable: false,
        };

        let settled = self
            .link
            .send_transfer_without_modifying_unsettled_map(
                &self.outgoing,
                transfer,
                payload.clone(),
            )
            .await?;

        match settled {
            true => {
                let _ = sender.send(None);
            }
            false => {
                let unsettled = UnsettledMessage::new(payload, None, message_format, sender);
                let mut guard = self.link.unsettled.write();
                guard
                    .get_or_insert(OrderedMap::new())
                    .insert(delivery_tag, unsettled);
            }
        }

        Ok(())
    }

    /// Re-send a source-only delivery as a new, non-resumed delivery.
    ///
    /// Unlike the other resumption outcomes, this consumes a fresh link-credit
    /// and assigns a new delivery-id/tag, so it only runs once the attach
    /// exchange has completed with complete maps.
    ///
    /// On failure the message is pushed back onto [`Self::pending_redeliveries`]
    /// so a later resume can retry it instead of losing its settlement channel.
    async fn resend(&mut self, unsettled_message: UnsettledMessage) -> Result<(), SendError> {
        let detached_fut = self.incoming.recv();
        let tag = match self
            .link
            .get_delivery_tag_or_detached(&self.outgoing, detached_fut)
            .await
        {
            Ok(tag) => tag,
            Err(error) => {
                self.pending_redeliveries.push(unsettled_message);
                return Err(error.into());
            }
        };
        let new_delivery_tag = DeliveryTag::from(tag);
        let transfer = match self.link.generate_non_resuming_transfer_performative(
            new_delivery_tag.clone(),
            unsettled_message.message_format,
            None,
            None,
            false,
        ) {
            Ok(transfer) => transfer,
            Err(error) => {
                self.pending_redeliveries.push(unsettled_message);
                return Err(error.into());
            }
        };

        match self
            .link
            .send_transfer_without_modifying_unsettled_map(
                &self.outgoing,
                transfer,
                unsettled_message.payload.clone(),
            )
            .await
        {
            Ok(true) => {
                let _ = unsettled_message.settle();
                Ok(())
            }
            Ok(false) => {
                self.link
                    .unsettled
                    .write()
                    .get_or_insert(OrderedMap::new())
                    .insert(new_delivery_tag, unsettled_message);
                Ok(())
            }
            Err(error) => {
                self.pending_redeliveries.push(unsettled_message);
                Err(error.into())
            }
        }
    }

    async fn resume_incoming_attach(
        &mut self,
        initial_remote_attach: Option<Attach>,
    ) -> Result<(), SenderResumeErrorKind> {
        self.reallocate_output_handle().await?;
        let mut incomplete_rounds = 0usize;

        // The peer-initiated attach, when present, was already received by the
        // caller: only the first exchange consumes it. After a §2.6.13 suspend
        // both sides exchange fresh attaches.
        let mut exchange = match initial_remote_attach {
            Some(remote_attach) => {
                self.link
                    .send_attach(&self.outgoing, &self.session, AttachMode::Resume)
                    .await?;
                self.link.on_incoming_attach(remote_attach)?
            }
            None => self.exchange_attach(AttachMode::Resume).await?,
        };

        loop {
            match exchange {
                SenderAttachExchange::Complete => break,
                SenderAttachExchange::IncompleteUnsettled(resuming_deliveries) => {
                    for (delivery_tag, resuming) in resuming_deliveries {
                        self.handle_resuming_delivery(delivery_tag, resuming)
                            .await?;
                    }

                    // The unsettled map was incomplete. AMQP 1.0 §2.6.13: after
                    // the reduction of state, the two parties suspend and
                    // re-attempt to resume the link.
                    if let Err(error) = self.detach_with_error(None).await {
                        // The peer closed the link while we were suspending
                        // it, so there is nothing left to resume. Drop the
                        // buffered redeliveries instead of losing them.
                        self.pending_redeliveries.clear();
                        return Err(SenderResumeErrorKind::DetachError(error));
                    }

                    incomplete_rounds += 1;
                    #[cfg(any(feature = "tracing", feature = "log"))]
                    let unsettled_len = self.link.unsettled.read().as_ref().map(|m| m.len());
                    #[cfg(feature = "tracing")]
                    tracing::debug!(
                        incomplete_rounds,
                        ?unsettled_len,
                        buffered = self.pending_redeliveries.len(),
                        "unsettled map incomplete; suspending and re-attempting"
                    );
                    #[cfg(feature = "log")]
                    log::debug!(
                        "unsettled map incomplete (round {}, local {:?}, buffered {}); suspending and re-attempting",
                        incomplete_rounds,
                        unsettled_len,
                        self.pending_redeliveries.len()
                    );

                    if incomplete_rounds > MAX_INCOMPLETE_UNSETTLED_ROUNDS {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(
                            incomplete_rounds,
                            "the unsettled map did not become complete; aborting resumption"
                        );
                        #[cfg(feature = "log")]
                        log::warn!(
                            "the unsettled map did not become complete after {} rounds; aborting resumption",
                            incomplete_rounds
                        );
                        // The public resume error enum on this branch has no
                        // dedicated variant; reuse the existing illegal-state
                        // detach error after logging the cause.
                        return Err(SenderResumeErrorKind::DetachError(
                            DetachError::IllegalState,
                        ));
                    }

                    // `detach_with_error` released the output handle and the
                    // session released the link relay, so re-register the link
                    // before re-attempting the attach exchange.
                    self.reallocate_output_handle().await?;
                    exchange = self.exchange_attach(AttachMode::Resume).await?;
                }
                SenderAttachExchange::Resume(resuming_deliveries) => {
                    for (delivery_tag, resuming) in resuming_deliveries {
                        self.handle_resuming_delivery(delivery_tag, resuming)
                            .await?;
                    }

                    // Both maps are complete; the exchange succeeded. Do not
                    // detach here: a complete-map exchange must not be
                    // suspended again, which caused repeated resend/detach
                    // loops.
                    break;
                }
            }
        }

        // Re-send the buffered source-only deliveries as new (non-resumed)
        // transfers. This also runs when the exchange completed with
        // `Complete` (both maps empty), so those deliveries are re-delivered
        // instead of being silently dropped. `resend()` re-queues the message
        // on failure, so the rest of the queue survives a failed drain.
        while let Some(unsettled_message) = self.pending_redeliveries.pop() {
            self.resend(unsettled_message).await?;
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
/// ```rust,ignore
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

    /// Get a reference to the link's source field
    pub fn source(&self) -> &Option<Source> {
        &self.inner.link.source
    }

    /// Get a mutable reference to the link's source field
    pub fn source_mut(&mut self) -> &mut Option<Source> {
        &mut self.inner.link.source
    }

    /// Get a reference to the link's target field
    pub fn target(&self) -> &Option<Target> {
        &self.inner.link.target
    }

    /// Get a mutable reference to the link's target field
    pub fn target_mut(&mut self) -> &mut Option<Target> {
        &mut self.inner.link.target
    }

    async fn resume_inner(mut self) -> Result<Sender, SenderResumeError> {
        try_as_sender!(self, self.inner.resume_incoming_attach(None).await);
        Ok(Sender { inner: self.inner })
    }

    /// Resume the sender link on the original session
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub async fn resume(self) -> Result<Sender, SenderResumeError> {
        self.resume_inner().await
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

    cfg_not_wasm32! {
        async fn resume_with_timeout_inner(
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

        /// Resume the sender link with a timeout
        #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
        pub async fn resume_with_timeout(
            self,
            duration: Duration,
        ) -> Result<Sender, SenderResumeError> {
            self.resume_with_timeout_inner(duration).await
        }

        async fn resume_incoming_attach_with_timeout_inner(
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

        /// Resume the sender link on the original session with an Attach sent by the remote peer
        pub async fn resume_incoming_attach_with_timeout(
            self,
            remote_attach: Attach,
            duration: Duration,
        ) -> Result<Sender, SenderResumeError> {
            self.resume_incoming_attach_with_timeout_inner(remote_attach, duration).await
        }
    }

    /// Resume the sender on a specific session
    pub async fn resume_on_session<R>(
        mut self,
        session: &SessionHandle<R>,
    ) -> Result<Sender, SenderResumeError> {
        self.inner.session = session.control.clone();
        self.inner.outgoing = session.outgoing.clone();
        self.resume_inner().await
    }

    /// Resume the sender on a specific session
    pub async fn resume_incoming_attach_on_session<R>(
        mut self,
        remote_attach: Attach,
        session: &SessionHandle<R>,
    ) -> Result<Sender, SenderResumeError> {
        self.inner.session = session.control.clone();
        self.inner.outgoing = session.outgoing.clone();

        try_as_sender!(
            self,
            self.inner.resume_incoming_attach(Some(remote_attach)).await
        );
        Ok(Sender { inner: self.inner })
    }

    cfg_not_wasm32! {
        /// Resume the sender on a specific session with timeout
        pub async fn resume_on_session_with_timeout<R>(
            mut self,
            session: &SessionHandle<R>,
            duration: Duration,
        ) -> Result<Sender, SenderResumeError> {
            self.inner.session = session.control.clone();
            self.inner.outgoing = session.outgoing.clone();
            self.resume_with_timeout_inner(duration).await
        }

        /// Resume the sender on a specific session with timeout
        pub async fn resume_incoming_attach_on_session_with_timeout<R>(
            mut self,
            remote_attach: Attach,
            session: &SessionHandle<R>,
            duration: Duration,
        ) -> Result<Sender, SenderResumeError> {
            self.inner.session = session.control.clone();
            self.inner.outgoing = session.outgoing.clone();
            self.resume_incoming_attach_with_timeout_inner(remote_attach, duration)
                .await
        }
    }
}
#[cfg(test)]
mod tests {
    use std::marker::PhantomData;
    use std::sync::Arc;

    use fe2o3_amqp_types::{
        definitions::{Handle, ReceiverSettleMode, SenderSettleMode},
        messaging::{DeliveryState, Received, Target},
        primitives::OrderedMap,
    };
    use parking_lot::RwLock;
    use tokio::sync::Notify;

    use super::*;
    use crate::endpoint::OutputHandle;
    use crate::link::state::{LinkFlowState, LinkFlowStateInner, LinkState};
    use crate::util::Consumer;

    type DeliveryOutcome = oneshot::Receiver<Option<DeliveryState>>;

    fn make_sender_inner_with_channels(
        max_frame_size: usize,
    ) -> (
        SenderInner<SenderLink<Target>>,
        mpsc::Receiver<SessionControl>,
        mpsc::Receiver<LinkFrame>,
        mpsc::Sender<LinkFrame>,
    ) {
        let (session_tx, session_rx) = mpsc::channel::<SessionControl>(16);
        let (outgoing_tx, outgoing_rx) = mpsc::channel::<LinkFrame>(16);
        let (incoming_tx, incoming_rx) = mpsc::channel::<LinkFrame>(16);

        let flow_state = Arc::new(LinkFlowState::sender(LinkFlowStateInner {
            initial_delivery_count: 0,
            delivery_count: 0,
            link_credit: 0,
            available: 0,
            drain: false,
            properties: None,
        }));
        let consumer = Consumer::new(Arc::new(Notify::new()), flow_state);
        let link = SenderLink::<Target> {
            role: PhantomData,
            local_state: LinkState::Attached,
            name: String::from("test-sender"),
            output_handle: Some(OutputHandle(0)),
            input_handle: None,
            snd_settle_mode: SenderSettleMode::Mixed,
            rcv_settle_mode: ReceiverSettleMode::First,
            source: None,
            target: None,
            max_message_size: 0,
            offered_capabilities: None,
            desired_capabilities: None,
            flow_state: consumer,
            unsettled: Arc::new(RwLock::new(None)),
            verify_incoming_source: true,
            verify_incoming_target: true,
        };
        let _ = max_frame_size;
        let inner = SenderInner {
            link,
            buffer_size: 16,
            session: session_tx,
            outgoing: outgoing_tx,
            incoming: incoming_rx,
            pending_redeliveries: Vec::new(),
        };
        (inner, session_rx, outgoing_rx, incoming_tx)
    }

    fn peer_receiver_attach() -> Attach {
        Attach {
            name: String::from("test-sender"),
            handle: Handle(0),
            role: fe2o3_amqp_types::definitions::Role::Receiver,
            snd_settle_mode: SenderSettleMode::Mixed,
            rcv_settle_mode: ReceiverSettleMode::First,
            source: None,
            target: Some(Box::new(
                fe2o3_amqp_types::messaging::TargetArchetype::Target(Target::default()),
            )),
            unsettled: None,
            incomplete_unsettled: false,
            initial_delivery_count: None,
            max_message_size: None,
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,
        }
    }

    fn seed_unsettled_entries(
        link: &SenderLink<Target>,
        n: u32,
        state: Option<DeliveryState>,
    ) -> Vec<DeliveryOutcome> {
        let mut map = OrderedMap::new();
        let mut receivers = Vec::with_capacity(n as usize);
        for i in 0..n {
            let (tx, rx) = oneshot::channel();
            receivers.push(rx);
            map.insert(
                DeliveryTag::from(i.to_be_bytes().to_vec()),
                UnsettledMessage::new(Bytes::new(), state.clone(), 0, tx),
            );
        }
        *link.unsettled.write() = Some(map);
        receivers
    }

    fn peer_attach_with_unsettled(
        remote: OrderedMap<DeliveryTag, Option<DeliveryState>>,
    ) -> Attach {
        let mut attach = peer_receiver_attach();
        attach.unsettled = Some(remote);
        attach.incomplete_unsettled = false;
        attach
    }

    /// A scripted peer for the incomplete-unsettled resume loop: it answers
    /// every `AllocateLink`/`GetMaxFrameSize`, replies to each `Attach` with
    /// `peer_attach`, and answers each `Detach` with a non-closing detach.
    async fn drive_resume_peer(
        mut session_rx: mpsc::Receiver<SessionControl>,
        mut outgoing_rx: mpsc::Receiver<LinkFrame>,
        initial_incoming_tx: mpsc::Sender<LinkFrame>,
        peer_attach: Attach,
    ) {
        let mut incoming_tx = initial_incoming_tx;
        loop {
            tokio::select! {
                ctrl = session_rx.recv() => match ctrl {
                    Some(SessionControl::AllocateLink { link_relay, responder, .. }) => {
                        incoming_tx = match link_relay {
                            LinkRelay::Sender { tx, .. } => tx,
                            LinkRelay::Receiver { tx, .. } => tx,
                        };
                        let _ = responder.send(Ok(OutputHandle(1)));
                    }
                    Some(SessionControl::GetMaxFrameSize(tx)) => {
                        // Small enough that a map of hundreds of entries is
                        // truncated, exercising the incomplete-unsettled path.
                        let _ = tx.send(512);
                    }
                    Some(_) => {}
                    None => break,
                },
                frame = outgoing_rx.recv() => match frame {
                    Some(LinkFrame::Attach(_)) => {
                        let _ = incoming_tx.send(LinkFrame::Attach(peer_attach.clone())).await;
                    }
                    Some(LinkFrame::Detach(_)) => {
                        let _ = incoming_tx
                            .send(LinkFrame::Detach(Detach {
                                handle: Handle(0),
                                closed: false,
                                error: None,
                            }))
                            .await;
                    }
                    Some(_) => {}
                    None => break,
                },
            }
        }
    }

    /// If the peer keeps advertising the same unsettled map and the reduction
    /// never shrinks it, resumption aborts after a bounded number of rounds
    /// instead of looping forever, while the already-buffered redeliveries
    /// survive for a later resume.
    #[tokio::test]
    async fn incomplete_unsettled_resume_aborts_and_keeps_pending_redeliveries() {
        let (mut inner, session_rx, outgoing_rx, incoming_tx) =
            make_sender_inner_with_channels(512);
        inner.link.local_state = LinkState::Detached;
        inner.link.flow_state().state().lock.write().link_credit = 100_000;

        let received = Some(DeliveryState::Received(Received {
            section_number: 0,
            section_offset: 0,
        }));
        let _rx = seed_unsettled_entries(&inner.link, 400, received.clone());

        // Half the tags are absent from the peer's map and are buffered for
        // redelivery. The other half keep the local map truncated, so the
        // exchange stays incomplete and eventually hits the round cap.
        let mut remote = OrderedMap::new();
        for i in 0..200u32 {
            remote.insert(
                DeliveryTag::from(i.to_be_bytes().to_vec()),
                received.clone(),
            );
        }
        let peer_attach = peer_attach_with_unsettled(remote);
        let peer = tokio::spawn(drive_resume_peer(
            session_rx,
            outgoing_rx,
            incoming_tx,
            peer_attach,
        ));

        let result = inner.resume_incoming_attach(None).await;
        peer.abort();

        assert!(
            matches!(
                result,
                Err(SenderResumeErrorKind::DetachError(
                    DetachError::IllegalState
                ))
            ),
            "unexpected result: {result:?}"
        );
        assert_eq!(inner.pending_redeliveries.len(), 200);
    }

    /// A failed redelivery is put back on the queue instead of losing its
    /// settlement channel.
    #[tokio::test]
    async fn failed_redelivery_is_requeued() {
        let (mut inner, _session_rx, _outgoing_rx, incoming_tx) =
            make_sender_inner_with_channels(4096);
        inner.link.flow_state().state().lock.write().link_credit = 1000;
        // Closing the incoming channel makes the biased select observe the
        // closed detach channel first.
        drop(incoming_tx);

        let (tx, _rx) = oneshot::channel();
        let message = UnsettledMessage::new(Bytes::new(), None, 0, tx);
        let result = inner.resend(message).await;

        assert!(result.is_err());
        assert_eq!(inner.pending_redeliveries.len(), 1);
    }

    /// Dropping the sender closes the buffered redeliveries' settlement
    /// channels instead of letting their futures hang.
    #[tokio::test]
    async fn sender_drop_closes_pending_redeliveries() {
        let (mut inner, _session_rx, _outgoing_rx, _incoming_tx) =
            make_sender_inner_with_channels(4096);

        let (tx, mut outcome) = oneshot::channel();
        inner
            .pending_redeliveries
            .push(UnsettledMessage::new(Bytes::new(), None, 0, tx));

        drop(inner);

        assert!(outcome.try_recv().is_err());
    }
}
