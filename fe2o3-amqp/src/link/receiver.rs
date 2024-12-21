//! Implementation of AMQP1.0 receiver

use std::sync::atomic::{AtomicU32, Ordering};

use fe2o3_amqp_types::{
    definitions::{self, DeliveryTag, Fields, SequenceNo},
    messaging::{
        Accepted, Address, DeliveryState, FromBody, Modified, Rejected, Released, Source, Target,
    },
    performatives::{Attach, Detach, Transfer},
};
use tokio::sync::mpsc;

cfg_not_wasm32! {
    use std::time::Duration;
    use tokio::time::{error::Elapsed, timeout};
}

use crate::{
    control::SessionControl,
    endpoint::{self, LinkAttach, LinkDetach, LinkExt},
    session::SessionHandle,
    Payload,
};

use super::{
    builder::{self, WithTarget, WithoutName, WithoutSource},
    delivery::{Delivery, DeliveryInfo},
    error::DetachError,
    incomplete_transfer::IncompleteTransfer,
    receiver_link::count_number_of_sections_and_offset,
    role,
    shared_inner::{LinkEndpointInner, LinkEndpointInnerDetach, LinkEndpointInnerReattach},
    ArcReceiverUnsettledMap, DetachThenResumeReceiverError, DispositionError, FlowError,
    IllegalLinkStateError, LinkFrame, LinkRelay, LinkStateError, ReceiverAttachError,
    ReceiverAttachExchange, ReceiverFlowState, ReceiverLink, ReceiverResumeError,
    ReceiverResumeErrorKind, ReceiverTransferError, RecvError, DEFAULT_CREDIT,
};

cfg_transaction! {
    use fe2o3_amqp_types::definitions::AmqpError;
}

#[cfg(docsrs)]
use fe2o3_amqp_types::{
    messaging::{AmqpSequence, AmqpValue, Batch, Body},
    primitives::{LazyValue, Value},
};

/// Credit mode for the link
#[derive(Debug, Clone)]
pub enum CreditMode {
    /// Manual mode will require the user to manually allocate credit whenever
    /// the available credits are depleted
    Manual,

    /// The receiver will automatically re-fill the credit
    Auto(SequenceNo),
}

impl Default for CreditMode {
    fn default() -> Self {
        // Default credit
        Self::Auto(DEFAULT_CREDIT)
    }
}

/// An AMQP1.0 receiver
///
/// # Attach a new receiver with default configurations
///
/// ```rust, ignore
/// let mut receiver = Receiver::attach(
///     &mut session,           // mutable reference to SessionHandle
///     "rust-receiver-link-1", // link name
///     "q1"                    // Source address
/// ).await.unwrap();
///
/// // Receiver defaults to `ReceiverSettleMode::First` which spontaneously settles incoming delivery
/// let delivery: Delivery<String> = receiver.recv::<String>().await.unwrap();
///
/// receiver.close().await.unwrap();
/// ```
///
/// ## Default configuration
///
/// | Field | Default Value |
/// |-------|---------------|
/// |`name`|`String::default()`|
/// |`snd_settle_mode`|`SenderSettleMode::Mixed`|
/// |`rcv_settle_mode`|`ReceiverSettleMode::First`|
/// |`source`|`None` |
/// |`target`| `Some(Target)` |
/// |`initial_delivery_count`| `0` |
/// |`max_message_size`| `None` |
/// |`offered_capabilities`| `None` |
/// |`desired_capabilities`| `None` |
/// |`Properties`| `None` |
/// |`buffer_size`| `u16::MAX` |
/// |`role`| `role::Sender` |
/// |`auto_accept`|`false`|
///
/// # Customize configuration with [`builder::Builder`]
///
/// ```rust, ignore
/// let mut receiver = Receiver::builder()
///     .name("rust-receiver-link-1")
///     .source("q1")
///     .attach(&mut session)
///     .receiver_settle_mode(ReceiverSettleMode::Second)
///     .await
///     .unwrap();
/// ```
#[derive(Debug)]
pub struct Receiver {
    pub(crate) inner: ReceiverInner<ReceiverLink<Target>>,
}

impl Receiver {
    /// Creates a builder for the [`Receiver`]
    pub fn builder(
    ) -> builder::Builder<role::ReceiverMarker, Target, WithoutName, WithoutSource, WithTarget>
    {
        builder::Builder::<role::ReceiverMarker, Target, _, _, _>::new()
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

    /// Get the current credit of the link
    pub fn credit_mode(&self) -> &CreditMode {
        &self.inner.credit_mode
    }

    /// Set the credit mode
    ///
    /// This will not send a flow to the remote peer even if credits in `CreditMode::Auto` is changed.
    pub fn set_credit_mode(&mut self, credit_mode: CreditMode) {
        self.inner.credit_mode = credit_mode;
    }

    /// Get the `auto_accept` field of receiver
    pub fn auto_accept(&self) -> bool {
        self.inner.auto_accept
    }

    /// Set `auto_accept` to `value`
    pub fn set_auto_accept(&mut self, value: bool) {
        self.inner.auto_accept = value;
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

    /// Attach the receiver link to a session with the default configuration
    /// with the `name` and `source` address set the specified value
    ///
    /// # Default configuration
    ///
    /// | Field | Default Value |
    /// |-------|---------------|
    /// |`name`|`String::default()`|
    /// |`snd_settle_mode`|`SenderSettleMode::Mixed`|
    /// |`rcv_settle_mode`|`ReceiverSettleMode::First`|
    /// |`source`|`None` |
    /// |`target`| `Some(Target)` |
    /// |`initial_delivery_count`| `0` |
    /// |`max_message_size`| `None` |
    /// |`offered_capabilities`| `None` |
    /// |`desired_capabilities`| `None` |
    /// |`Properties`| `None` |
    /// |`buffer_size`| `u16::MAX` |
    /// |`role`| `role::Sender` |
    /// |`auto_accept`|`false`|
    ///  
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut receiver = Receiver::attach(
    ///     &mut session,           // mutable reference to SessionHandle
    ///     "rust-receiver-link-1", // link name
    ///     "q1"                    // Source address
    /// ).await.unwrap();
    /// ```
    pub async fn attach<R>(
        session: &mut SessionHandle<R>,
        name: impl Into<String>,
        addr: impl Into<Address>,
    ) -> Result<Receiver, ReceiverAttachError> {
        Self::builder()
            .name(name)
            .source(addr)
            .attach(session)
            .await
    }

    /// Receive a message from the link
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let delivery: Delivery<String> = receiver.recv::<String>().await.unwrap();
    /// receiver.accept(&delivery).await.unwrap();
    /// ```
    ///
    /// # The receive type `Delivery<T>`
    ///
    /// If the user is not certain the exact type (including the exact body section type) to
    /// receive, [`Body<Value>`] is probably the safest bet. [`Body<T>`] covers all possible body
    /// section types, including an empty body, and `Value` covers all possible AMQP 1.0 types.
    ///
    /// ```rust,ignore
    /// // `Body<Value>` covers all possibilities
    /// let delivery: Delivery<Body<Value>> = receiver.recv::<Body<Value>>().await.unwrap();
    /// receiver.accept(&delivery).await.unwrap();
    /// ```
    ///
    /// Lazy deserialization is supported with [`Body<LazyValue>`]. The [`LazyValue`] type is a
    /// a thin wrapper around the encoded bytes and can be deserialized lazily.
    ///
    /// ```rust,ignore
    /// // `Body<LazyValue>` covers all possibilities and can be deserialized lazily
    /// let delivery: Delivery<Body<LazyValue>> = receiver.recv::<Body<LazyValue>>().await.unwrap();
    /// receiver.accept(&delivery).await.unwrap();
    /// ```
    ///
    /// If the user is certain an [`AmqpValue`] body section is expected, then the user could use
    /// [`AmqpValue<KnownType>`] if the exact message type `KnownType` is known and implements
    /// [`serde::Deserialize`]. If the user is not sure about the exact message type, one could use
    /// [`AmqpValue<Value>`] to cover the most general cases. This also applies to [`AmqpSequence`]
    /// or [`Batch<AmqpSequence>`].
    ///
    /// ```rust,ignore
    /// // `KnownType` must implement `serde::Deserialize`
    /// let delivery: Delivery<AmqpValue<KnownType>> = receiver.recv::<AmqpValue<KnownType>>().await.unwrap();
    /// receiver.accept(&delivery).await.unwrap();
    /// ```
    ///
    /// Another option to use a custom type is to implement the [`FromBody`] trait on a custom type.
    ///
    /// ```rust,ignore
    /// #[derive(Deserialize)]
    /// struct Foo {
    ///     a: i32
    /// }
    ///
    /// impl FromBody<'_> for Foo {
    ///     type Body = AmqpValue<Foo>;
    ///
    ///     fn from_body(body: Self::Body) -> Self {
    ///         body.0
    ///     }
    /// }
    ///
    /// let delivery: Delivery<Foo> = receiver.recv::<Foo>().await.unwrap();
    /// receiver.accept(&delivery).await.unwrap();
    /// ```
    ///
    /// # Cancel safety
    ///
    /// This function is cancel-safe. See [#22](https://github.com/minghuaw/fe2o3-amqp/issues/22)
    /// for more details.
    pub async fn recv<T>(&mut self) -> Result<Delivery<T>, RecvError>
    where
        for<'de> T: FromBody<'de> + Send,
    {
        self.inner.recv().await
    }

    /// Set the link credit. This will stop draining if the link is in a draining cycle
    pub async fn set_credit(&mut self, credit: SequenceNo) -> Result<(), IllegalLinkStateError> {
        self.inner.set_credit(credit).await
    }

    /// Drain the link.
    ///
    /// This will send a `Flow` performative with the `drain` field set to true.
    /// Setting the credit will set the `drain` field to false and stop draining
    pub async fn drain(&mut self) -> Result<(), IllegalLinkStateError> {
        self.inner.drain().await
    }

    /// Send the link properties to the remote peer via a `Flow` performative
    pub async fn send_properties(&self) -> Result<(), FlowError> {
        self.inner.send_properties().await
    }

    /// Detach the link.
    ///
    /// This will send a `Detach` performative with the `closed` field set to false. If the remote
    /// peer responds with a Detach performative whose `closed` field is set to true, the link will
    /// re-attach and then close by exchanging closing Detach performatives.
    pub async fn detach(mut self) -> Result<DetachedReceiver, (DetachedReceiver, DetachError)> {
        match self.inner.detach_with_error(None).await {
            Ok(_) => Ok(DetachedReceiver { inner: self.inner }),
            Err(err) => Err((DetachedReceiver { inner: self.inner }, err)),
        }
    }

    /// Detach the link with an error.
    ///
    /// This will send a `Detach` performative with the `closed` field set to false. If the remote
    /// peer responds with a Detach performative whose `closed` field is set to true, the link will
    /// re-attach and then close by exchanging closing Detach performatives.
    pub async fn detach_with_error(
        mut self,
        error: impl Into<definitions::Error>,
    ) -> Result<DetachedReceiver, (DetachedReceiver, DetachError)> {
        match self.inner.detach_with_error(Some(error.into())).await {
            Ok(_) => Ok(DetachedReceiver { inner: self.inner }),
            Err(err) => Err((DetachedReceiver { inner: self.inner }, err)),
        }
    }

    cfg_not_wasm32! {
        /// Detach the link with a timeout
        ///
        /// This simply wraps [`detach`](#method.detach) with a `timeout`
        pub async fn detach_with_timeout(
            self,
            duration: Duration,
        ) -> Result<Result<DetachedReceiver, (DetachedReceiver, DetachError)>, Elapsed> {
            timeout(duration, self.detach()).await
        }
    }

    /// Detach the link and then resume on a new session.
    ///
    /// This will still attemt to re-attach even if the detach fails.
    /// `DetachThenResumeReceiverError::Resume` will be returned if detach succeeds but re-attach
    /// fails. `DetachThenResumeReceiverError::Detach` will be returned if both detach and re-attach
    /// fails.
    pub async fn detach_then_resume_on_session<R>(
        &mut self,
        new_session: &SessionHandle<R>,
    ) -> Result<ReceiverAttachExchange, DetachThenResumeReceiverError> {
        // detach the link
        let detach_result = self
            .inner
            .detach_with_error(None)
            .await
            .map_err(DetachThenResumeReceiverError::from);

        let is_reattaching = !self.inner.session.same_channel(&new_session.control);

        // re-attach the link
        self.inner.session = new_session.control.clone();
        self.inner.outgoing = new_session.outgoing.clone();
        let exchange_result = self
            .inner
            .resume_incoming_attach(None, is_reattaching)
            .await
            .map_err(DetachThenResumeReceiverError::from);

        match (detach_result, exchange_result) {
            (_, Ok(exchange)) => Ok(exchange),
            (Ok(_), Err(err)) => Err(err),
            (Err(err), Err(_)) => Err(err),
        }
    }

    /// Close the link.
    ///
    /// This will send a Detach performative with the `closed` field set to true.
    pub async fn close(mut self) -> Result<(), DetachError> {
        self.inner.close_with_error(None).await
    }

    /// Close the link with an error.
    ///
    /// This will send a Detach performative with the `closed` field set to true.
    pub async fn close_with_error(
        mut self,
        error: impl Into<definitions::Error>,
    ) -> Result<(), DetachError> {
        // Stop link transfer before closing
        self.set_credit(0).await?;
        self.inner.close_with_error(Some(error.into())).await
    }

    /// Accept the message by sending a disposition with the `delivery_state` field set
    /// to `Accept`.
    ///
    /// This will not send disposition if the delivery is not found in the local unsettled map.
    ///
    /// # Example
    ///
    /// The code of the example below can be found in the [GitHub repo](https://github.com/minghuaw/fe2o3-amqp/blob/main/examples/receiver/src/main.rs)
    ///
    /// ```rust,ignore
    /// let delivery: Delivery<Value> = receiver.recv().await.unwrap();
    /// receiver.accept(&delivery).await.unwrap();
    /// ```
    pub async fn accept(
        &self,
        delivery_info: impl Into<DeliveryInfo>,
    ) -> Result<(), DispositionError> {
        let state = TerminalDeliveryState::Accepted(Accepted {});
        self.dispose(delivery_info, state).await
    }

    /// Accept the message by sending one or more disposition(s) with the `delivery_state` field set
    /// to `Accept`
    ///
    /// Only deliveries that are found in the local unsettled map will be included in the disposition frame(s).
    ///
    /// # Example
    ///
    /// The code of the example below can be found in the [GitHub repo](https://github.com/minghuaw/fe2o3-amqp/blob/main/examples/dispose_multiple/src/main.rs)
    ///
    /// ```rust,ignore
    /// let delivery1: Delivery<Value> = receiver.recv().await.unwrap();
    /// let delivery2: Delivery<Value> = receiver.recv().await.unwrap();
    /// receiver.accept_all(vec![&delivery1, &delivery2]).await.unwrap();
    /// ```
    pub async fn accept_all(
        &self,
        deliveries: impl IntoIterator<Item = impl Into<DeliveryInfo>>,
    ) -> Result<(), DispositionError> {
        let state = TerminalDeliveryState::Accepted(Accepted {});
        self.dispose_all(deliveries, state).await
    }

    /// Reject the message by sending a disposition with the `delivery_state` field set
    /// to `Reject`
    ///
    /// This will not send disposition if the delivery is not found in the local unsettled map.
    pub async fn reject(
        &self,
        delivery_info: impl Into<DeliveryInfo>,
        error: impl Into<Option<definitions::Error>>,
    ) -> Result<(), DispositionError> {
        let state = TerminalDeliveryState::Rejected(Rejected {
            error: error.into(),
        });
        self.dispose(delivery_info, state).await
    }

    /// Reject the message by sending one or more disposition(s) with the `delivery_state` field set
    /// to `Reject`
    ///
    /// Only deliveries that are found in the local unsettled map will be included in the disposition frame(s).
    pub async fn reject_all(
        &self,
        deliveries: impl IntoIterator<Item = impl Into<DeliveryInfo>>,
        error: impl Into<Option<definitions::Error>>,
    ) -> Result<(), DispositionError> {
        let state = TerminalDeliveryState::Rejected(Rejected {
            error: error.into(),
        });
        self.dispose_all(deliveries, state).await
    }

    /// Release the message by sending a disposition with the `delivery_state` field set
    /// to `Release`
    ///
    /// This will not send disposition if the delivery is not found in the local unsettled map.
    pub async fn release(
        &self,
        delivery_info: impl Into<DeliveryInfo>,
    ) -> Result<(), DispositionError> {
        let state = TerminalDeliveryState::Released(Released {});
        self.dispose(delivery_info, state).await
    }

    /// Release the message by sending one or more disposition(s) with the `delivery_state` field set
    /// to `Release`
    ///
    /// Only deliveries that are found in the local unsettled map will be included in the disposition frame(s).
    pub async fn release_all(
        &self,
        deliveries: impl IntoIterator<Item = impl Into<DeliveryInfo>>,
    ) -> Result<(), DispositionError> {
        let state = TerminalDeliveryState::Released(Released {});
        self.dispose_all(deliveries, state).await
    }

    /// Modify the message by sending a disposition with the `delivery_state` field set
    /// to `Modify`
    ///
    /// This will not send disposition if the delivery is not found in the local unsettled map.
    pub async fn modify(
        &self,
        delivery_info: impl Into<DeliveryInfo>,
        modified: Modified,
    ) -> Result<(), DispositionError> {
        let state = TerminalDeliveryState::Modified(modified);
        self.dispose(delivery_info, state).await
    }

    /// Modify the message by sending one or more disposition(s) with the `delivery_state` field set
    /// to `Modify`
    ///
    /// Only deliveries that are found in the local unsettled map will be included in the disposition frame(s).
    pub async fn modify_all(
        &self,
        deliveries: impl IntoIterator<Item = impl Into<DeliveryInfo>>,
        modified: Modified,
    ) -> Result<(), DispositionError> {
        let state = TerminalDeliveryState::Modified(modified);
        self.dispose_all(deliveries, state).await
    }

    /// Dispose the message by sending a disposition with the provided state
    ///
    /// This will not send disposition if the delivery is not found in the local unsettled map.
    pub async fn dispose(
        &self,
        delivery_info: impl Into<DeliveryInfo>,
        state: impl Into<TerminalDeliveryState>,
    ) -> Result<(), DispositionError> {
        let state: TerminalDeliveryState = state.into();
        self.inner.dispose(delivery_info, None, state.into()).await
    }

    /// Dispose the message by sending one or more disposition(s) with the provided state
    ///
    /// Only deliveries that are found in the local unsettled map will be included in the disposition frame(s).
    pub async fn dispose_all(
        &self,
        deliveries: impl IntoIterator<Item = impl Into<DeliveryInfo>>,
        state: impl Into<TerminalDeliveryState>,
    ) -> Result<(), DispositionError> {
        let state: TerminalDeliveryState = state.into();
        let delivery_infos = deliveries.into_iter().map(|d| d.into()).collect();
        self.inner
            .dispose_all(delivery_infos, None, state.into())
            .await
    }
}

#[derive(Debug, Clone)]
/// Terminal delivery states that can be used by the receiver to dispose of a delivery
pub enum TerminalDeliveryState {
    /// 3.4.2 Accepted
    Accepted(Accepted),

    /// 3.4.3 Rejected
    Rejected(Rejected),

    /// 3.4.4 Released
    Released(Released),

    /// 3.4.5 Modified
    Modified(Modified),
}

impl From<TerminalDeliveryState> for DeliveryState {
    fn from(value: TerminalDeliveryState) -> Self {
        match value {
            TerminalDeliveryState::Accepted(val) => Self::Accepted(val),
            TerminalDeliveryState::Rejected(val) => Self::Rejected(val),
            TerminalDeliveryState::Released(val) => Self::Released(val),
            TerminalDeliveryState::Modified(val) => Self::Modified(val),
        }
    }
}

impl TryFrom<DeliveryState> for TerminalDeliveryState {
    type Error = DeliveryState;

    fn try_from(value: DeliveryState) -> Result<Self, Self::Error> {
        match value {
            DeliveryState::Accepted(val) => Ok(Self::Accepted(val)),
            DeliveryState::Rejected(val) => Ok(Self::Rejected(val)),
            DeliveryState::Released(val) => Ok(Self::Released(val)),
            DeliveryState::Modified(val) => Ok(Self::Modified(val)),
            _ => Err(value),
        }
    }
}

impl From<Accepted> for TerminalDeliveryState {
    fn from(value: Accepted) -> Self {
        Self::Accepted(value)
    }
}

impl From<Rejected> for TerminalDeliveryState {
    fn from(value: Rejected) -> Self {
        Self::Rejected(value)
    }
}

impl From<Released> for TerminalDeliveryState {
    fn from(value: Released) -> Self {
        Self::Released(value)
    }
}

impl From<Modified> for TerminalDeliveryState {
    fn from(value: Modified) -> Self {
        Self::Modified(value)
    }
}

#[derive(Debug)]
pub(crate) struct ReceiverInner<L: endpoint::ReceiverLink> {
    pub(crate) link: L,
    pub(crate) buffer_size: usize,
    pub(crate) credit_mode: CreditMode,
    pub(crate) processed: AtomicU32, // SequenceNo,
    pub(crate) auto_accept: bool,

    // Control sender to the session
    pub(crate) session: mpsc::Sender<SessionControl>,

    // Outgoing mpsc channel to send the Link Frames
    pub(crate) outgoing: mpsc::Sender<LinkFrame>,
    pub(crate) incoming: mpsc::Receiver<LinkFrame>,

    // Wrap in a box to avoid clippy warning large_enum_variant on link acceptor's output
    pub(crate) incomplete_transfer: Option<Box<IncompleteTransfer>>,
}

impl<L: endpoint::ReceiverLink> Drop for ReceiverInner<L> {
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

impl<L> LinkEndpointInner for ReceiverInner<L>
where
    L: endpoint::ReceiverLink<AttachError = ReceiverAttachError, DetachError = DetachError>
        + LinkExt<FlowState = ReceiverFlowState, Unsettled = ArcReceiverUnsettledMap>
        + LinkAttach<AttachExchange = ReceiverAttachExchange>
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
        LinkRelay::Receiver {
            tx,
            output_handle: (),
            flow_state: self.link.flow_state().clone(),
            unsettled: self.link.unsettled().clone(),
            receiver_settle_mode: self.link.rcv_settle_mode().clone(),
            // This only controls whether a multi-transfer delivery id
            // will be added to sessions map
            more: false,
        }
    }

    fn session_control(&self) -> &mpsc::Sender<SessionControl> {
        &self.session
    }

    async fn exchange_attach(
        &mut self,
        is_reattaching: bool,
    ) -> Result<ReceiverAttachExchange, <Self::Link as LinkAttach>::AttachError> {
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

    /// # Cancel safety
    async fn send_detach(
        &mut self,
        closed: bool,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError> {
        self.link.send_detach(&self.outgoing, closed, error).await // cancel safe
    }
}

impl<L> LinkEndpointInnerReattach for ReceiverInner<L>
where
    L: endpoint::ReceiverLink<AttachError = ReceiverAttachError, DetachError = DetachError>
        + LinkExt<FlowState = ReceiverFlowState, Unsettled = ArcReceiverUnsettledMap>
        + LinkAttach<AttachExchange = ReceiverAttachExchange>
        + Send
        + Sync,
{
    fn handle_reattach_outcome(
        &mut self,
        outcome: ReceiverAttachExchange,
    ) -> Result<&mut Self, L::AttachError> {
        match outcome {
            ReceiverAttachExchange::Complete => Ok(self),
            //  Re-attach should have None valued unsettled, so this should be invalid
            ReceiverAttachExchange::IncompleteUnsettled | ReceiverAttachExchange::Resume => {
                Err(ReceiverAttachError::IllegalState)
            }
        }
    }
}

impl<L> ReceiverInner<L>
where
    L: endpoint::ReceiverLink<
            FlowError = IllegalLinkStateError,
            TransferError = ReceiverTransferError,
            DispositionError = IllegalLinkStateError,
            AttachError = ReceiverAttachError,
            DetachError = DetachError,
        > + LinkExt<FlowState = ReceiverFlowState, Unsettled = ArcReceiverUnsettledMap>
        + LinkAttach<AttachExchange = ReceiverAttachExchange>
        + Send
        + Sync,
{
    pub(crate) async fn recv<T>(&mut self) -> Result<Delivery<T>, RecvError>
    where
        for<'de> T: FromBody<'de> + Send,
    {
        loop {
            match self.recv_inner().await? // FIXME: cancel safe? if oneshot channel is cancel safe
            {
                Some(delivery) => return Ok(delivery),
                None => continue, // Incomplete transfer, there are more transfer frames coming
            }
        }
    }

    /// # Cancel safety
    ///
    /// This should be cancel safe if oneshot channel is cancel safe
    #[inline]
    pub(crate) async fn recv_inner<T>(&mut self) -> Result<Option<Delivery<T>>, RecvError>
    where
        for<'de> T: FromBody<'de> + Send,
    {
        let frame = self
            .incoming
            .recv()
            .await // cancel safe
            .ok_or(LinkStateError::IllegalSessionState)?;

        match frame {
            LinkFrame::Detach(detach) => {
                let closed = detach.closed;
                self.link.send_detach(&self.outgoing, closed, None).await?; // cancel safe
                self.link
                    .on_incoming_detach(detach)
                    .map_err(Into::into)
                    .and_then(|_| match closed {
                        true => Err(LinkStateError::RemoteClosed.into()),
                        false => Err(LinkStateError::RemoteDetached.into()),
                    })
            }
            LinkFrame::Transfer {
                input_handle: _,
                performative,
                payload,
            } => self.on_incoming_transfer(performative, payload).await, // cancel safe
            LinkFrame::Attach(_) => Err(LinkStateError::IllegalState.into()),
            LinkFrame::Flow(_) | LinkFrame::Disposition(_) => {
                // Flow and Disposition are handled by LinkRelay which runs
                // in the session loop
                unreachable!()
            }
            #[cfg(feature = "transaction")]
            LinkFrame::Acquisition(_) => {
                let error = definitions::Error::new(
                    AmqpError::NotImplemented,
                    "Transactional acquisition is not implemented".to_string(),
                    None,
                );
                self.close_with_error(Some(error)).await?; // FIXME: cancel safe? if oneshot chanenl is cancel safe
                Err(RecvError::TransactionalAcquisitionIsNotImeplemented)
            }
        }
    }

    fn on_transfer_state(
        &mut self,
        delivery_tag: &Option<DeliveryTag>,
        settled: Option<bool>,
        state: DeliveryState,
    ) -> Result<(), RecvError> {
        match &mut self.incomplete_transfer {
            Some(incomplete) if *delivery_tag == incomplete.performative.delivery_tag => {
                if let DeliveryState::Received(received) = &state {
                    incomplete.keep_buffer_till_section_number_and_offset(
                        received.section_number,
                        received.section_offset,
                    );
                }
            }
            Some(_) | None => {}
        }

        self.link
            .on_transfer_state(delivery_tag, settled, state)
            .map_err(Into::into)
    }

    fn on_incomplete_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<(), RecvError> {
        // Partial transfer of the delivery
        match &mut self.incomplete_transfer {
            Some(incomplete) => {
                incomplete.or_assign(transfer)?;
                incomplete.append(payload);

                if let Some(delivery_tag) = incomplete.performative.delivery_tag.clone() {
                    // Update unsettled map in the link
                    self.link.on_incomplete_transfer(
                        delivery_tag,
                        incomplete.section_number.unwrap_or(0),
                        incomplete.section_offset,
                    );
                }
            }
            None => {
                let incomplete = IncompleteTransfer::new(transfer, payload);
                if let Some(delivery_tag) = incomplete.performative.delivery_tag.clone() {
                    // Update unsettled map in the link
                    self.link.on_incomplete_transfer(
                        delivery_tag,
                        incomplete.section_number.unwrap_or(0),
                        incomplete.section_offset,
                    );
                }
                self.incomplete_transfer = Some(Box::new(incomplete));
            }
        }

        Ok(())
    }

    /// # Cancel safety
    ///
    /// This is cancel safe because all internal `.await` point(s) are cancel safe
    async fn on_resuming_transfer<T>(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<Delivery<T>>, RecvError>
    where
        for<'de> T: FromBody<'de> + Send,
    {
        // need to check whether the incoming transfer matches
        match (
            &transfer.delivery_tag,
            self.incomplete_transfer
                .as_ref()
                .map(|i| &i.performative.delivery_tag),
        ) {
            (Some(remote), Some(Some(local))) => {
                // The transfer does not belong to the buffer incomplete transfer
                if remote != local {
                    let (section_number, section_offset) =
                        count_number_of_sections_and_offset(&payload);
                    let delivery = self.link.on_complete_transfer(
                        transfer,
                        &payload,
                        section_number,
                        section_offset,
                    )?;

                    // Auto accept the message and leave settled to be determined based on rcv_settle_mode
                    if self.auto_accept {
                        self.dispose(&delivery, None, Accepted {}.into()).await?;
                        // cancel safe
                    }

                    Ok(Some(delivery))
                } else {
                    // The new Transfer belongs to the buffered incomplete transfer
                    self.on_complete_transfer(transfer, payload).await // cancel safe
                }
            }
            _ => {
                // The new Transfer belongs to the buffered incomplete transfer that there isn't an incomplete_transfer
                self.on_complete_transfer(transfer, payload).await // cancel safe
            }
        }
    }

    /// # Cancel safety
    ///
    /// This is cancel safe because all internal `.await` point(s) are cancel safe
    async fn on_complete_transfer<T>(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<Delivery<T>>, RecvError>
    where
        for<'de> T: FromBody<'de> + Send,
    {
        let delivery = match self.incomplete_transfer.take() {
            Some(mut incomplete) => {
                incomplete.or_assign(transfer)?;
                incomplete.append(payload); // This also computes the section number and offset incrementally

                self.link.on_complete_transfer(
                    incomplete.performative,
                    incomplete.buffer,
                    incomplete.section_number.unwrap_or(0),
                    incomplete.section_offset,
                )?
            }
            None => {
                let (section_number, section_offset) =
                    count_number_of_sections_and_offset(&payload);
                self.link.on_complete_transfer(
                    transfer,
                    &payload,
                    section_number,
                    section_offset,
                )?
            }
        };

        // Auto accept the message and leave settled to be determined based on rcv_settle_mode
        if self.auto_accept {
            self.dispose(&delivery, None, Accepted {}.into()).await?; // cancel safe
        }

        Ok(Some(delivery))
    }

    /// # Cancel safety
    ///
    /// This is cancel safe because all internal `.await` point(s) are cancel safe
    #[inline]
    async fn on_incoming_transfer<T>(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<Delivery<T>>, RecvError>
    where
        for<'de> T: FromBody<'de> + Send,
    {
        // Aborted messages SHOULD be discarded by the recipient (any payload
        // within the frame carrying the performative MUST be ignored). An aborted
        // message is implicitly settled
        if transfer.aborted {
            let _ = self.incomplete_transfer.take();
            return Ok(None);
        }

        if let Some(state) = transfer.state.clone() {
            // Setting the state
            // on the transfer can be thought of as being equivalent to sending a disposition immediately before
            // the transfer performative, i.e., it is the state of the delivery (not the transfer) that existed at the
            // point the frame was sent.
            self.on_transfer_state(&transfer.delivery_tag, transfer.settled, state)?;
        }

        if transfer.more {
            // Partial transfer of the delivery
            // There is only ONE incomplet transfer locally, so the partial transfer must belong to the
            // same delivery
            self.on_incomplete_transfer(transfer, payload)?;
            // Partial delivery doesn't yield a complete message
            Ok(None)
        } else if transfer.resume {
            self.on_resuming_transfer(transfer, payload).await // cancel safe
        } else {
            // Final transfer of the delivery
            self.on_complete_transfer(transfer, payload).await // cancel safe
        }
    }

    /// Set the link credit. This will stop draining if the link is in a draining cycle
    ///
    /// # Cancel safety
    ///
    /// This is cancel safe as internanlly it only `.await` on sending over `tokio::mpsc::Sender`
    #[inline]
    pub async fn set_credit(&mut self, credit: SequenceNo) -> Result<(), IllegalLinkStateError> {
        self.processed = AtomicU32::new(0);
        if let CreditMode::Auto(_) = self.credit_mode {
            self.credit_mode = CreditMode::Auto(credit)
        }

        self.link
            .send_flow(&self.outgoing, Some(credit), Some(false), false, false)
            .await // cancel safe
    }

    /// This is cancel safe because all internal `.await` points are cancel safe
    #[inline]
    pub(crate) async fn dispose(
        &self,
        delivery_info: impl Into<DeliveryInfo>,
        settled: Option<bool>,
        state: DeliveryState,
    ) -> Result<(), DispositionError> {
        let delivery_info = delivery_info.into();
        self.link
            .dispose(&self.outgoing, delivery_info, settled, state, false)
            .await?; // cancel safe

        let prev = self.processed.fetch_add(1, Ordering::Release);
        self.update_credit_if_auto(prev + 1).await?; // cancel safe
        Ok(())
    }

    /// This is cancel safe because all internal `.await` points are cancel safe
    #[inline]
    pub(crate) async fn dispose_all(
        &self,
        delivery_infos: Vec<DeliveryInfo>,
        settled: Option<bool>,
        state: DeliveryState,
    ) -> Result<(), DispositionError> {
        let total = delivery_infos.len() as u32;
        self.link
            .dispose_all(&self.outgoing, delivery_infos, settled, state, false)
            .await?; // cancel safe

        let prev = self.processed.fetch_add(total, Ordering::Release);
        self.update_credit_if_auto(prev + total).await?; // cancel safe
        Ok(())
    }

    /// This is cancel safe because it only `.await` on a cancel safe future
    #[inline]
    async fn update_credit_if_auto(&self, processed: u32) -> Result<(), DispositionError> {
        if let CreditMode::Auto(max_credit) = self.credit_mode {
            if processed >= max_credit / 2 {
                // Reset link credit
                self.processed.swap(0, Ordering::Release);
                self.link
                    .send_flow(&self.outgoing, Some(max_credit), Some(false), false, false)
                    .await?; // cancel safe
            }
        }
        Ok(())
    }

    /// Drain the link.
    ///
    /// This will send a `Flow` performative with the `drain` field set to true.
    /// Setting the credit will set the `drain` field to false and stop draining
    #[inline]
    pub async fn drain(&mut self) -> Result<(), DispositionError> {
        self.processed = AtomicU32::new(0);

        // Return if already draining
        if self.link.flow_state().drain() {
            return Ok(());
        }

        // Send a flow with Drain set to true
        self.link
            .send_flow(&self.outgoing, None, Some(true), false, false)
            .await
    }

    /// Send the properties of the link via a Flow frame
    #[inline]
    pub async fn send_properties(&self) -> Result<(), FlowError> {
        self.link
            .send_flow(&self.outgoing, None, None, false, true)
            .await
    }
}

impl ReceiverInner<ReceiverLink<Target>> {
    pub(crate) async fn resume_incoming_attach(
        &mut self,
        mut initial_remote_attach: Option<Attach>,
        is_reattaching: bool,
    ) -> Result<ReceiverAttachExchange, ReceiverResumeErrorKind> {
        self.reallocate_output_handle().await?;

        let exchange = match initial_remote_attach.take() {
            Some(remote_attach) => {
                self.link
                    .send_attach(&self.outgoing, &self.session, is_reattaching)
                    .await?;
                self.link.on_incoming_attach(remote_attach)?
            }
            None => self.exchange_attach(is_reattaching).await?,
        };
        #[cfg(feature = "tracing")]
        tracing::debug!(?exchange);
        #[cfg(feature = "log")]
        log::debug!("exchange = {:?}", exchange);

        let credit = self.link.flow_state.link_credit();
        self.set_credit(credit).await?;

        Ok(exchange)
    }
}

/// A detached receiver
///
/// # Example
///
/// Link re-attachment
///
/// ```rust,ignore
/// let detached = receiver.detach().await.unwrap();
/// let resuming_receiver = detached.resume().await.unwrap();
/// ```
#[derive(Debug)]
pub struct DetachedReceiver {
    inner: ReceiverInner<ReceiverLink<Target>>,
}

macro_rules! try_as_recver {
    ($self:ident, $f:expr) => {
        match $f {
            Ok(outcome) => outcome,
            Err(error) => {
                return Err(ReceiverResumeError {
                    detached_recver: $self,
                    kind: error.into(),
                })
            }
        }
    };
}

/// The outcome of a resuming receiver
#[derive(Debug)]
pub enum ResumingReceiver {
    /// The resumption is complete with no unsettled deliveries
    Complete(Receiver),

    /// At least one side sent an Attach with an incomplete unsettled map
    ///
    /// Please note that additional detach-resume may be necessary when there are
    /// unsettled deliveries
    IncompleteUnsettled(Receiver),

    /// The link is attached to resume partial deliveries
    ///
    /// The current implementation only allows one partial delivery
    ///
    /// Please note that additional detach-resume may be necessary when there are
    /// unsettled deliveries
    Resume(Receiver),
}

impl ResumingReceiver {
    /// Returns `Ok(Receiver)` if value is `Complete` otherwise returns `op(self)`
    pub fn complete_or<E>(self, err: E) -> Result<Receiver, E> {
        match self {
            ResumingReceiver::Complete(receiver) => Ok(receiver),
            _ => Err(err),
        }
    }

    /// Returns `Ok(Receiver)` if value is `Complete` otherwise returns `op(self)`
    pub fn complete_or_else<F, E>(self, op: F) -> Result<Receiver, E>
    where
        F: FnOnce(Self) -> E,
    {
        match self {
            ResumingReceiver::Complete(receiver) => Ok(receiver),
            _ => Err((op)(self)),
        }
    }

    /// Consumes the enum and get the receiver
    pub fn into_receiver(self) -> Receiver {
        match self {
            ResumingReceiver::Complete(receiver) => receiver,
            ResumingReceiver::IncompleteUnsettled(receiver) => receiver,
            ResumingReceiver::Resume(receiver) => receiver,
        }
    }

    /// Get a reference to the receiver
    pub fn as_receiver(&self) -> &Receiver {
        self.as_ref()
    }

    /// Get a mutable reference to the receiver
    pub fn as_receiver_mut(&mut self) -> &mut Receiver {
        self.as_mut()
    }
}

impl AsRef<Receiver> for ResumingReceiver {
    fn as_ref(&self) -> &Receiver {
        match self {
            ResumingReceiver::Complete(receiver) => receiver,
            ResumingReceiver::IncompleteUnsettled(receiver) => receiver,
            ResumingReceiver::Resume(receiver) => receiver,
        }
    }
}

impl AsMut<Receiver> for ResumingReceiver {
    fn as_mut(&mut self) -> &mut Receiver {
        match self {
            ResumingReceiver::Complete(receiver) => receiver,
            ResumingReceiver::IncompleteUnsettled(receiver) => receiver,
            ResumingReceiver::Resume(receiver) => receiver,
        }
    }
}

impl From<ResumingReceiver> for Receiver {
    fn from(value: ResumingReceiver) -> Self {
        value.into_receiver()
    }
}

impl DetachedReceiver {
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

    async fn resume_inner(
        mut self,
        is_reattaching: bool,
    ) -> Result<ResumingReceiver, ReceiverResumeError> {
        let exchange = try_as_recver!(
            self,
            self.inner
                .resume_incoming_attach(None, is_reattaching)
                .await
        );
        let receiver = Receiver { inner: self.inner };
        let resuming_receiver = match exchange {
            ReceiverAttachExchange::Complete => ResumingReceiver::Complete(receiver),
            ReceiverAttachExchange::IncompleteUnsettled => {
                ResumingReceiver::IncompleteUnsettled(receiver)
            }
            ReceiverAttachExchange::Resume => ResumingReceiver::Resume(receiver),
        };
        Ok(resuming_receiver)
    }

    /// Resume the receiver link
    ///
    /// Please note that the link may need to be detached and then resume multiple
    /// times if there are unsettled deliveries.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub async fn resume(self) -> Result<ResumingReceiver, ReceiverResumeError> {
        self.resume_inner(false).await
    }

    cfg_not_wasm32! {
        async fn resume_with_timeout_inner(
            mut self,
            duration: Duration,
            is_reattaching: bool,
        ) -> Result<ResumingReceiver, ReceiverResumeError> {
            let fut = self.inner.resume_incoming_attach(None, is_reattaching);

            match tokio::time::timeout(duration, fut).await {
                Ok(Ok(exchange)) => {
                    let receiver = Receiver { inner: self.inner };
                    let resuming_receiver = match exchange {
                        ReceiverAttachExchange::Complete => ResumingReceiver::Complete(receiver),
                        ReceiverAttachExchange::IncompleteUnsettled => {
                            ResumingReceiver::IncompleteUnsettled(receiver)
                        }
                        ReceiverAttachExchange::Resume => ResumingReceiver::Resume(receiver),
                    };
                    Ok(resuming_receiver)
                }
                Ok(Err(kind)) => Err(ReceiverResumeError {
                    detached_recver: self,
                    kind,
                }),
                Err(_) => {
                    try_as_recver!(self, self.inner.detach_with_error(None).await);
                    Err(ReceiverResumeError {
                        detached_recver: self,
                        kind: ReceiverResumeErrorKind::Timeout,
                    })
                }
            }
        }

        /// Resume the receiver link with a timeout.
        ///
        /// Upon failure, the detached receiver can be accessed via `error.detached_recver`
        ///
        /// Please note that the link may need to be detached and then resume multiple
        /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
        #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
        pub async fn resume_with_timeout(
            self,
            duration: Duration,
        ) -> Result<ResumingReceiver, ReceiverResumeError> {
            self.resume_with_timeout_inner(duration, false).await
        }
    }

    /// Resume the receiver on a specific session
    ///
    /// Please note that the link may need to be detached and then resume multiple
    /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
    pub async fn resume_on_session<R>(
        mut self,
        session: &SessionHandle<R>,
    ) -> Result<ResumingReceiver, ReceiverResumeError> {
        let is_reattaching = !self.inner.session.same_channel(&session.control);

        self.inner.session = session.control.clone();
        self.inner.outgoing = session.outgoing.clone();

        self.resume_inner(is_reattaching).await
    }

    /// Resume the receiver link on the original session with an Attach sent by the remote peer
    ///
    /// Please note that the link may need to be detached and then resume multiple
    /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
    pub async fn resume_incoming_attach(
        mut self,
        remote_attach: Attach,
    ) -> Result<ResumingReceiver, ReceiverResumeError> {
        let exchange = try_as_recver!(
            self,
            self.inner
                .resume_incoming_attach(Some(remote_attach), false)
                .await
        );
        let receiver = Receiver { inner: self.inner };
        let resuming_receiver = match exchange {
            ReceiverAttachExchange::Complete => ResumingReceiver::Complete(receiver),
            ReceiverAttachExchange::IncompleteUnsettled => {
                ResumingReceiver::IncompleteUnsettled(receiver)
            }
            ReceiverAttachExchange::Resume => ResumingReceiver::Resume(receiver),
        };
        Ok(resuming_receiver)
    }

    /// Resume the receiver on a specific session
    ///
    /// Please note that the link may need to be detached and then resume multiple
    /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
    pub async fn resume_incoming_attach_on_session<R>(
        mut self,
        remote_attach: Attach,
        session: &SessionHandle<R>,
    ) -> Result<ResumingReceiver, ReceiverResumeError> {
        let is_reattaching = !self.inner.session.same_channel(&session.control);

        self.inner.session = session.control.clone();
        self.inner.outgoing = session.outgoing.clone();

        let exchange = try_as_recver!(
            self,
            self.inner
                .resume_incoming_attach(Some(remote_attach), is_reattaching)
                .await
        );
        let receiver = Receiver { inner: self.inner };
        let resuming_receiver = match exchange {
            ReceiverAttachExchange::Complete => ResumingReceiver::Complete(receiver),
            ReceiverAttachExchange::IncompleteUnsettled => {
                ResumingReceiver::IncompleteUnsettled(receiver)
            }
            ReceiverAttachExchange::Resume => ResumingReceiver::Resume(receiver),
        };
        Ok(resuming_receiver)
    }

    cfg_not_wasm32! {
        /// Resume the receiver on a specific session with timeout
        ///
        /// Please note that the link may need to be detached and then resume multiple
        /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
        pub async fn resume_on_session_with_timeout<R>(
            mut self,
            session: &SessionHandle<R>,
            duration: Duration,
        ) -> Result<ResumingReceiver, ReceiverResumeError> {
            let is_reattaching = !self.inner.session.same_channel(&session.control);
            self.inner.session = session.control.clone();
            self.inner.outgoing = session.outgoing.clone();
            self.resume_with_timeout_inner(duration, is_reattaching).await
        }

        /// Resume the receiver link on the original session with an Attach sent by the remote peer
        ///
        /// Please note that the link may need to be detached and then resume multiple
        /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
        pub async fn resume_incoming_attach_with_timeout(
            mut self,
            remote_attach: Attach,
            duration: Duration,
        ) -> Result<ResumingReceiver, ReceiverResumeError> {
            let fut = self.inner.resume_incoming_attach(Some(remote_attach), false);

            match tokio::time::timeout(duration, fut).await {
                Ok(Ok(exchange)) => {
                    let receiver = Receiver { inner: self.inner };
                    let resuming_receiver = match exchange {
                        ReceiverAttachExchange::Complete => ResumingReceiver::Complete(receiver),
                        ReceiverAttachExchange::IncompleteUnsettled => {
                            ResumingReceiver::IncompleteUnsettled(receiver)
                        }
                        ReceiverAttachExchange::Resume => ResumingReceiver::Resume(receiver),
                    };
                    Ok(resuming_receiver)
                }
                Ok(Err(kind)) => Err(ReceiverResumeError {
                    detached_recver: self,
                    kind,
                }),
                Err(_) => {
                    try_as_recver!(self, self.inner.detach_with_error(None).await);
                    Err(ReceiverResumeError {
                        detached_recver: self,
                        kind: ReceiverResumeErrorKind::Timeout,
                    })
                }
            }
        }

        /// Resume the receiver on a specific session with timeout
        ///
        /// Please note that the link may need to be detached and then resume multiple
        /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
        pub async fn resume_incoming_attach_on_session_with_timeout<R>(
            mut self,
            remote_attach: Attach,
            session: &SessionHandle<R>,
            duration: Duration,
        ) -> Result<ResumingReceiver, ReceiverResumeError> {
            let is_reattaching = !self.inner.session.same_channel(&session.control);

            self.inner.session = session.control.clone();
            self.inner.outgoing = session.outgoing.clone();

            let fut = self.inner.resume_incoming_attach(Some(remote_attach), is_reattaching);

            match tokio::time::timeout(duration, fut).await {
                Ok(Ok(exchange)) => {
                    let receiver = Receiver { inner: self.inner };
                    let resuming_receiver = match exchange {
                        ReceiverAttachExchange::Complete => ResumingReceiver::Complete(receiver),
                        ReceiverAttachExchange::IncompleteUnsettled => {
                            ResumingReceiver::IncompleteUnsettled(receiver)
                        }
                        ReceiverAttachExchange::Resume => ResumingReceiver::Resume(receiver),
                    };
                    Ok(resuming_receiver)
                }
                Ok(Err(kind)) => Err(ReceiverResumeError {
                    detached_recver: self,
                    kind,
                }),
                Err(_) => {
                    try_as_recver!(self, self.inner.detach_with_error(None).await);
                    Err(ReceiverResumeError {
                        detached_recver: self,
                        kind: ReceiverResumeErrorKind::Timeout,
                    })
                }
            }
        }
    }
}
