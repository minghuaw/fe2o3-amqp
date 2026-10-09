//! Implementation of AMQP1.0 receiver

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use fe2o3_amqp_types::{
    definitions::{
        self, DeliveryTag, Fields, Handle, LinkError, ReceiverSettleMode, Role, SequenceNo,
    },
    messaging::{
        Accepted, Address, DeliveryState, FromBody, Modified, Rejected, Released, Source, Target,
    },
    performatives::{Attach, Detach, Disposition, Transfer},
};
use tokio::sync::mpsc;

cfg_not_wasm32! {
    use std::time::Duration;
    use tokio::time::{error::Elapsed, timeout};
}

use crate::{
    control::SessionControl,
    endpoint::{self, LinkAttach, LinkDetach, LinkExt, LinkFlow, OutputHandle},
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
    ArcReceiverUnsettledMap, DetachThenResumeReceiverError, DispositionError, FlowError, LinkFrame,
    LinkOutcome, LinkRelay, LinkStateError, MessageSizeExceeded, ReceiverAttachError,
    ReceiverAttachExchange, ReceiverFlowState, ReceiverLink, ReceiverResumeError,
    ReceiverResumeErrorKind, ReceiverTransferError, RecvError, SessionOutcome, DEFAULT_CREDIT,
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
    ///
    /// When this value is set, deliveries larger than it are rejected on
    /// receive with `amqp:link:message-size-exceeded` and [`recv`](#method.recv)
    /// returns [`RecvError::MessageSizeExceeded`]; the link itself is not
    /// detached.
    pub fn max_message_size(&self) -> Option<u64> {
        self.inner.link.max_message_size()
    }

    /// Get the configured credit mode of the link
    pub fn credit_mode(&self) -> &CreditMode {
        &self.inner.credit_mode
    }

    /// Set the credit mode
    ///
    /// This will not send a flow to the remote peer even if credits in `CreditMode::Auto` is changed.
    pub fn set_credit_mode(&mut self, credit_mode: CreditMode) {
        self.inner.credit_mode = credit_mode;
    }

    /// Get the current credit of the link
    pub fn credit(&self) -> u32 {
        self.inner.link.flow_state.link_credit()
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
    /// # Message size
    ///
    /// When the link advertises a `max_message_size` (see
    /// [`max_message_size`](#method.max_message_size)) and the peer sends a
    /// delivery larger than it, the link is detached with the
    /// `amqp:link:message-size-exceeded` error condition (AMQP 1.0 §2.6.5) and
    /// this method returns [`RecvError::MessageSizeExceeded`]. The link can
    /// only be restored by resuming it.
    ///
    /// # Malformed deliveries
    ///
    /// The delivery-id and delivery-tag MUST be present on the first transfer
    /// of a delivery, and a continuation that carries them MUST repeat the same
    /// values (AMQP 1.0 §2.7.5). The transfer field text scopes the MUST to
    /// multi-transfer deliveries, but this crate requires them on the first
    /// transfer of any delivery, matching go-amqp and Qpid Proton. A violation
    /// detaches the link with `amqp:not-allowed` (AMQP 1.0 §2.6.5), so this
    /// method returns an error and the link is no longer usable.
    ///
    /// # Resumed deliveries
    ///
    /// A resumed delivery whose tag is not in the link's local unsettled map
    /// is ignored (AMQP 1.0 §2.6.13).
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
    pub async fn set_credit(&mut self, credit: SequenceNo) -> Result<(), LinkStateError> {
        self.inner.set_credit(credit).await
    }

    /// Drain the link.
    ///
    /// This will send a `Flow` performative with the `drain` field set to true.
    /// Setting the credit will set the `drain` field to false and stop draining
    pub async fn drain(&mut self) -> Result<(), LinkStateError> {
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
    pub async fn detach(
        mut self,
    ) -> Result<(DetachedReceiver, LinkOutcome), (DetachedReceiver, DetachError)> {
        match self.inner.detach_with_error(None).await {
            Ok(status) => Ok((
                DetachedReceiver {
                    inner: Box::new(self.inner),
                },
                status,
            )),
            Err(err) => Err((
                DetachedReceiver {
                    inner: Box::new(self.inner),
                },
                err,
            )),
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
    ) -> Result<(DetachedReceiver, LinkOutcome), (DetachedReceiver, DetachError)> {
        match self.inner.detach_with_error(Some(error.into())).await {
            Ok(status) => Ok((
                DetachedReceiver {
                    inner: Box::new(self.inner),
                },
                status,
            )),
            Err(err) => Err((
                DetachedReceiver {
                    inner: Box::new(self.inner),
                },
                err,
            )),
        }
    }

    cfg_not_wasm32! {
        /// Detach the link with a timeout
        ///
        /// This simply wraps [`detach`](#method.detach) with a `timeout`
        pub async fn detach_with_timeout(
            self,
            duration: Duration,
        ) -> Result<
            Result<(DetachedReceiver, LinkOutcome), (DetachedReceiver, DetachError)>,
            Elapsed,
        > {
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
        let detach_result = self.inner.detach_with_error(None).await;

        // If the peer closed the link instead of suspending it, the link can
        // no longer be resumed. The status is carried by the resume error.
        if let Ok(status) = &detach_result {
            if status.is_closed() {
                return Err(DetachThenResumeReceiverError::Resume(
                    ReceiverResumeErrorKind::LinkDetached(status.clone()),
                ));
            }
        }

        // re-attach the link
        self.inner.switch_session(new_session);
        let exchange_result = self.inner.resume_incoming_attach(None).await;

        match (detach_result, exchange_result) {
            (_, Ok(exchange)) => Ok(exchange),
            (Ok(_), Err(err)) => Err(DetachThenResumeReceiverError::Resume(err)),
            (Err(err), Err(_)) => Err(DetachThenResumeReceiverError::Detach(err)),
        }
    }

    /// Close the link.
    ///
    /// This will send a Detach performative with the `closed` field set to true.
    /// The returned [`LinkOutcome`] carries the error the peer attached to
    /// its closing detach, if any.
    pub async fn close(mut self) -> Result<LinkOutcome, DetachError> {
        self.inner.close_with_error(None).await
    }

    /// Close the link with an error.
    ///
    /// This will send a Detach performative with the `closed` field set to true.
    pub async fn close_with_error(
        mut self,
        error: impl Into<definitions::Error>,
    ) -> Result<LinkOutcome, DetachError> {
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

    /// Returns a [`ReceiverDisposer`] that can settle deliveries received by this
    /// receiver without holding the receiver itself.
    ///
    /// The disposer shares the underlying AMQP link state (unsettled map, flow state,
    /// outgoing channel) with the receiver. Settlement calls on the disposer are
    /// lock-free with respect to `recv()` — concurrent disposal and receive are safe.
    pub fn disposer(&self) -> ReceiverDisposer {
        ReceiverDisposer {
            outgoing: self.inner.outgoing.clone(),
            unsettled: self.inner.link.unsettled.clone(),
            rcv_settle_mode: self.inner.link.rcv_settle_mode.clone(),
            flow_state: self.inner.link.flow_state.clone(),
            output_handle: self.inner.link.output_handle.clone(),
            processed: Arc::clone(&self.inner.processed),
            credit_mode: self.inner.credit_mode.clone(),
            session_stop_reason: self.inner.link.session_stop_reason.clone(),
        }
    }
}

/// A lightweight helper that can settle deliveries previously received on a [`Receiver`].
///
/// Obtained via [`Receiver::disposer()`]. Settlement operations on this type do NOT
/// acquire the mutex guarding `recv()`, so calling them concurrently with an ongoing
/// receive is fully safe and will not block.
///
/// Cloning a `ReceiverDisposer` is cheap: all internal state is reference-counted.
#[derive(Clone, Debug)]
pub struct ReceiverDisposer {
    outgoing: mpsc::Sender<LinkFrame>,
    unsettled: ArcReceiverUnsettledMap,
    rcv_settle_mode: ReceiverSettleMode,
    flow_state: ReceiverFlowState,
    output_handle: Option<OutputHandle>,
    processed: Arc<AtomicU32>,
    credit_mode: CreditMode,
    session_stop_reason: Arc<OnceLock<SessionOutcome>>,
}

impl ReceiverDisposer {
    /// Accept the delivery (Accepted disposition).
    pub async fn accept(
        &self,
        delivery_info: impl Into<DeliveryInfo>,
    ) -> Result<(), DispositionError> {
        let info = delivery_info.into();
        self.dispose(info, DeliveryState::Accepted(Accepted {}))
            .await
    }

    /// Release the delivery (Released disposition — equivalent to `abandon`).
    pub async fn release(
        &self,
        delivery_info: impl Into<DeliveryInfo>,
    ) -> Result<(), DispositionError> {
        let info = delivery_info.into();
        self.dispose(info, DeliveryState::Released(Released {}))
            .await
    }

    async fn dispose(
        &self,
        delivery_info: DeliveryInfo,
        state: DeliveryState,
    ) -> Result<(), DispositionError> {
        let settled = match delivery_info
            .rcv_settle_mode
            .as_ref()
            .unwrap_or(&self.rcv_settle_mode)
        {
            ReceiverSettleMode::First => true,
            ReceiverSettleMode::Second => false,
        };

        let unsettled_state = if settled {
            let mut lock = self.unsettled.write();
            lock.as_mut()
                .and_then(|map| map.swap_remove(&delivery_info.delivery_tag))
        } else {
            let mut lock = self.unsettled.write();
            lock.get_or_insert(Default::default())
                .insert(delivery_info.delivery_tag.clone(), Some(state.clone()))
        };

        if unsettled_state.is_some() {
            let disposition = Disposition {
                role: Role::Receiver,
                first: delivery_info.delivery_id,
                last: None,
                settled,
                state: Some(state),
                batchable: false,
            };
            self.outgoing
                .send(LinkFrame::Disposition(disposition))
                .await
                .map_err(|_| match self.session_stop_reason.get() {
                    Some(reason) => DispositionError::SessionStopped(reason.clone()),
                    None => DispositionError::IllegalState, // defensive: no stop reason recorded; failure is link-local
                })?;
        }

        let prev = self.processed.fetch_add(1, Ordering::Release);
        self.refresh_credit_if_needed(prev + 1).await
    }

    async fn refresh_credit_if_needed(&self, processed: u32) -> Result<(), DispositionError> {
        if let CreditMode::Auto(max_credit) = self.credit_mode {
            if processed >= max_credit / 2 {
                self.processed.store(0, Ordering::Release);
                let handle: Handle = self
                    .output_handle
                    .clone()
                    .ok_or(DispositionError::IllegalState)?
                    .into();
                let delivery_count = {
                    let mut guard = self.flow_state.lock.write();
                    guard.link_credit = max_credit;
                    guard.drain = false;
                    guard.delivery_count
                };
                let flow = LinkFlow {
                    handle,
                    delivery_count: Some(delivery_count),
                    link_credit: Some(max_credit),
                    available: None,
                    drain: false,
                    echo: false,
                    properties: None,
                };
                self.outgoing
                    .send(LinkFrame::Flow(flow))
                    .await
                    .map_err(|_| match self.session_stop_reason.get() {
                        Some(reason) => DispositionError::SessionStopped(reason.clone()),
                        None => DispositionError::IllegalState, // defensive: no stop reason recorded; failure is link-local
                    })?;
            }
        }
        Ok(())
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
    pub(crate) processed: Arc<AtomicU32>, // SequenceNo,
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
        // A detach the relay already answered may be waiting in the engine's
        // channel. Apply it so the link state matches the detach; and once
        // any detach was drained, the peer has already detached the link, so
        // the closing detach this drop would otherwise send would be a
        // duplicate.
        let mut remote_detach_received = false;
        while let Ok(frame) = self.incoming.try_recv() {
            if let LinkFrame::Detach(detach) = frame {
                remote_detach_received = true;
                // If the state change fails, ignore it: the engine is being
                // dropped anyway.
                if self.link.apply_remote_detach_outcome(detach).is_err() {
                    #[cfg(feature = "tracing")]
                    tracing::debug!("failed to apply remote detach outcome on receiver drop");
                    #[cfg(feature = "log")]
                    log::debug!("failed to apply remote detach outcome on receiver drop");
                }
            }
            // Any other frame (e.g. a partially received transfer or an attach
            // response left behind by an interrupted reattach) is superseded
            // by the drop.
        }

        if !remote_detach_received {
            if let Some(handle) = self.link.output_handle_mut().take() {
                let detach = Detach {
                    handle: handle.into(),
                    closed: true,
                    error: None,
                };
                if let Err(_error) = self.outgoing.try_send(LinkFrame::Detach(detach)) {
                    #[cfg(any(feature = "log", feature = "tracing"))]
                    {
                        let reason = match &_error {
                            tokio::sync::mpsc::error::TrySendError::Full(_) => {
                                "control channel is full"
                            }
                            tokio::sync::mpsc::error::TrySendError::Closed(_) => {
                                "control channel is closed"
                            }
                        };
                        #[cfg(feature = "tracing")]
                        tracing::warn!(reason, "Failed to enqueue Detach frame on receiver drop");
                        #[cfg(feature = "log")]
                        log::warn!("Failed to enqueue Detach frame on receiver drop: {reason}");
                    }
                }
            }
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

    fn session_stop_reason(&self) -> &Arc<OnceLock<SessionOutcome>> {
        self.link().session_stop_reason()
    }

    async fn exchange_attach(
        &mut self,
    ) -> Result<ReceiverAttachExchange, <Self::Link as LinkAttach>::AttachError> {
        self.link
            .exchange_attach(&self.outgoing, &mut self.incoming)
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

/// Whether a transfer carries the delivery-id and delivery-tag required on the
/// first transfer of a delivery. This is stricter than the transfer field text,
/// which scopes the MUST to multi-transfer deliveries, but matches go-amqp and
/// Qpid Proton, which require the fields on the first transfer of any delivery.
fn ensure_delivery_identity(transfer: &Transfer) -> Result<(), ReceiverTransferError> {
    if transfer.delivery_id.is_none() {
        return Err(ReceiverTransferError::DeliveryIdIsNone);
    }
    if transfer.delivery_tag.is_none() {
        return Err(ReceiverTransferError::DeliveryTagIsNone);
    }
    Ok(())
}

impl<L> ReceiverInner<L>
where
    L: endpoint::ReceiverLink<
            FlowError = LinkStateError,
            TransferError = ReceiverTransferError,
            DispositionError = LinkStateError,
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
            match self.recv_inner().await? {
                Some(delivery) => return Ok(delivery),
                None => continue, // Incomplete transfer, there are more transfer frames coming
            }
        }
    }

    /// # Cancel safety
    ///
    /// This is cancel safe because all internal `.await` point(s) are cancel
    /// safe (`tokio::sync::mpsc` operations and synchronous processing)
    #[inline]
    pub(crate) async fn recv_inner<T>(&mut self) -> Result<Option<Delivery<T>>, RecvError>
    where
        for<'de> T: FromBody<'de> + Send,
    {
        // When the session or the connection stops, the channel closes and this
        // returns `RecvError::LinkStateError(SessionStopped(reason))` with the
        // stop reason observed by the link.
        let frame = match self.incoming.recv().await {
            // cancel safe
            Some(frame) => frame,
            None => {
                return Err(match self.link().session_stop_reason().get() {
                    Some(reason) => {
                        RecvError::LinkStateError(LinkStateError::SessionStopped(reason.clone()))
                    }
                    // defensive: no stop reason recorded; failure is link-local
                    None => RecvError::LinkStateError(LinkStateError::IllegalState),
                });
            }
        };

        match frame {
            // The relay already sent the reply to this peer detach; the link
            // records the outcome here. Reported as-is even when the session
            // is stopping: a stop without a detach shows up as the channel
            // closing (`None` above).
            LinkFrame::Detach(detach) => match self.link.apply_remote_detach_outcome(detach) {
                Ok(status) => Err(RecvError::LinkDetached(status)),
                Err(err) => Err(RecvError::LinkStateError(err.into())),
            },
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
                // Best-effort close; the acquisition error below is what the
                // caller sees.
                let _ = self.close_with_error(Some(error)).await;
                Err(RecvError::TransactionalAcquisitionIsNotImeplemented)
            }
        }
    }

    /// Adjust the buffered incomplete delivery and the link's unsettled map
    /// for a transfer that carries a delivery state.
    ///
    /// A continuation transfer may omit the delivery tag; when it does, the
    /// buffered delivery's tag is used so that the state is attributed to the
    /// delivery being assembled.
    async fn on_transfer_state(
        &mut self,
        delivery_tag: &Option<DeliveryTag>,
        settled: Option<bool>,
        state: DeliveryState,
    ) -> Result<(), RecvError> {
        let effective_tag = match delivery_tag {
            Some(tag) => Some(tag.clone()),
            None => self
                .incomplete_transfer
                .as_ref()
                .map(|incomplete| incomplete.delivery_tag().clone()),
        };
        let Some(effective_tag) = effective_tag else {
            // A state-carrying transfer with no buffered delivery and no
            // delivery-tag cannot be attributed to a delivery.
            return self
                .close_on_malformed_delivery(ReceiverTransferError::DeliveryTagIsNone)
                .await;
        };

        if let Some(incomplete) = &mut self.incomplete_transfer {
            let belongs = delivery_tag
                .as_ref()
                .is_none_or(|remote| incomplete.delivery_tag() == remote);
            if belongs {
                if let DeliveryState::Received(received) = &state {
                    incomplete.keep_buffer_till_section_number_and_offset(
                        received.section_number,
                        received.section_offset,
                    );
                }
            }
        }

        self.link
            .on_transfer_state(&Some(effective_tag), settled, state)
            .map_err(Into::into)
    }

    /// Close the link because a transfer violated the multi-frame delivery
    /// rules. AMQP 1.0 §2.6.5 requires the endpoint to be detached with error
    /// information and then destroyed.
    async fn close_on_malformed_delivery(
        &mut self,
        error: ReceiverTransferError,
    ) -> Result<(), RecvError> {
        let detach_error = definitions::Error::new(
            definitions::AmqpError::NotAllowed,
            Some(error.to_string()),
            None,
        );
        self.close_with_error(Some(detach_error)).await?;
        Err(error.into())
    }

    async fn on_incomplete_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<(), RecvError> {
        // Partial transfer of the delivery
        let (delivery_tag, section_number, section_offset) =
            if let Some(incomplete) = &mut self.incomplete_transfer {
                if let Err(error) = incomplete.try_append(transfer, payload) {
                    return self.close_on_malformed_delivery(error).await;
                }
                (
                    incomplete.delivery_tag().clone(),
                    incomplete.section_number(),
                    incomplete.section_offset(),
                )
            } else {
                match IncompleteTransfer::start(transfer, payload) {
                    Ok(incomplete) => {
                        let bookkeeping = (
                            incomplete.delivery_tag().clone(),
                            incomplete.section_number(),
                            incomplete.section_offset(),
                        );
                        self.incomplete_transfer = Some(Box::new(incomplete));
                        bookkeeping
                    }
                    Err(error) => return self.close_on_malformed_delivery(error).await,
                }
            };

        // Update the unsettled map in the link
        self.link
            .on_incomplete_transfer(delivery_tag, section_number, section_offset);

        Ok(())
    }

    /// Complete a reassociated delivery that has no buffered chunks, leaving
    /// any other buffered delivery untouched.
    ///
    /// The tag is known to be in the local unsettled map: resumed deliveries
    /// that are not are ignored before dispatch.
    ///
    /// # Cancel safety
    ///
    /// This is cancel safe because all internal `.await` point(s) are cancel safe
    async fn on_reassociated_transfer<T>(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<Delivery<T>>, RecvError>
    where
        for<'de> T: FromBody<'de> + Send,
    {
        // A different, previously unsettled delivery is being reassociated
        if let Err(error) = ensure_delivery_identity(&transfer) {
            return self.close_on_malformed_delivery(error).await.map(|()| None);
        }

        let (section_number, section_offset) = count_number_of_sections_and_offset(&payload);
        let delivery =
            self.link
                .on_complete_transfer(transfer, &payload, section_number, section_offset)?;

        // Auto accept the message and leave settled to be determined based on rcv_settle_mode
        if self.auto_accept {
            self.dispose(&delivery, None, Accepted {}.into()).await?;
            // cancel safe
        }

        Ok(Some(delivery))
    }

    /// The bytes of the delivery accumulated so far that belong to the same
    /// delivery as `transfer` (i.e. the buffered chunks of the incomplete
    /// multi-frame delivery, when the transfer is a continuation of it).
    fn accumulated_message_size(&self, transfer: &Transfer) -> u64 {
        match &self.incomplete_transfer {
            Some(incomplete) if incomplete.is_same_delivery_as(transfer) => {
                incomplete.accumulated_payload_size()
            }
            _ => 0,
        }
    }

    /// Whether the transfer names a delivery in the local unsettled map with a
    /// non-terminal state. AMQP 1.0 §2.6.13 requires the receiver to ignore
    /// resumed deliveries that are not in its local unsettled map.
    fn is_known_unsettled(&self, transfer: &Transfer) -> bool {
        let Some(tag) = transfer.delivery_tag.as_ref() else {
            return false;
        };
        let guard = self.link.unsettled().read();
        guard.as_ref().is_some_and(|map| {
            map.get(tag)
                .is_some_and(|state| state.as_ref().is_none_or(|state| !state.is_terminal()))
        })
    }

    /// Detach the link because a delivery exceeds the negotiated
    /// max-message-size, as required for a `message-size-exceeded` link error
    /// (AMQP 1.0 §2.6.5). The buffered chunks are discarded and the link can
    /// only be restored by resuming it.
    async fn close_on_message_size_exceeded(
        &mut self,
        total_size: u64,
        max_size: u64,
    ) -> RecvError {
        #[cfg(feature = "tracing")]
        tracing::warn!(
            "Detaching link: received message of {total_size} bytes exceeds the max message size of {max_size}"
        );
        #[cfg(feature = "log")]
        log::warn!(
            "Detaching link: received message of {total_size} bytes exceeds the max message size of {max_size}"
        );

        self.incomplete_transfer.take();

        let error = definitions::Error::new(
            LinkError::MessageSizeExceeded,
            Some(format!(
                "received message larger than max size of {max_size}"
            )),
            None,
        );
        match self.close_with_error(Some(error)).await {
            Ok(_) => RecvError::MessageSizeExceeded(MessageSizeExceeded {
                size: total_size,
                max_size,
            }),
            Err(detach_error) => detach_error.into(),
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
        let delivery = if let Some(mut incomplete) = self.incomplete_transfer.take() {
            // A final transfer that does not belong to the buffered delivery
            // is rejected (AMQP 1.0 §2.6.14 forbids interleaving on a link).
            if let Err(error) = incomplete.try_append(transfer, payload) {
                return self.close_on_malformed_delivery(error).await.map(|()| None);
            }

            let (performative, buffer, section_number, section_offset) = incomplete.into_parts();
            self.link
                .on_complete_transfer(performative, buffer, section_number, section_offset)?
        } else {
            // A single-frame delivery is identified by its delivery-id/tag.
            if let Err(error) = ensure_delivery_identity(&transfer) {
                return self.close_on_malformed_delivery(error).await.map(|()| None);
            }

            let (section_number, section_offset) = count_number_of_sections_and_offset(&payload);
            self.link
                .on_complete_transfer(transfer, &payload, section_number, section_offset)?
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
        let matches_buffer = self
            .incomplete_transfer
            .as_ref()
            .is_some_and(|incomplete| incomplete.is_same_delivery_as(&transfer));

        // Aborted messages SHOULD be discarded by the recipient (any payload
        // within the frame carrying the performative MUST be ignored). An aborted
        // message is implicitly settled
        if transfer.aborted {
            // An aborted transfer that explicitly names a different delivery
            // while one is being assembled violates AMQP 1.0 §2.6.14.
            if self.incomplete_transfer.is_some() && !matches_buffer {
                return self
                    .close_on_malformed_delivery(
                        ReceiverTransferError::InconsistentFieldInMultiFrameDelivery,
                    )
                    .await
                    .map(|()| None);
            }

            // An aborted message is implicitly settled: discard the buffered
            // chunks and the entry the partial frames added to the unsettled
            // map. A completed delivery (no buffer) is left untouched.
            if let Some(incomplete) = self.incomplete_transfer.take() {
                let mut guard = self.link.unsettled().write();
                if let Some(map) = guard.as_mut() {
                    let _ = map.swap_remove(incomplete.delivery_tag());
                }
            }
            return Ok(None);
        }

        // AMQP 1.0 §2.6.13: the receiver MUST ignore resumed deliveries that
        // are not in its local unsettled map.
        if transfer.resume && !matches_buffer && !self.is_known_unsettled(&transfer) {
            return Ok(None);
        }

        if let Some(state) = transfer.state.clone() {
            // Setting the state
            // on the transfer can be thought of as being equivalent to sending a disposition immediately before
            // the transfer performative, i.e., it is the state of the delivery (not the transfer) that existed at the
            // point the frame was sent.
            self.on_transfer_state(&transfer.delivery_tag, transfer.settled, state)
                .await?;
        }

        // Enforce the negotiated max-message-size of the link on every
        // transfer frame: detach the link with the
        // `amqp:link:message-size-exceeded` error condition as soon as the
        // accumulated message size would exceed it.
        if let Some(max_size) = self.link.max_message_size() {
            let total = self.accumulated_message_size(&transfer) + payload.len() as u64;
            if total > max_size {
                return Err(self.close_on_message_size_exceeded(total, max_size).await);
            }
        }

        if transfer.more {
            // Partial transfer of the delivery; it does not yield a message
            self.on_incomplete_transfer(transfer, payload)
                .await
                .map(|()| None)
        } else if transfer.resume && !matches_buffer {
            // A resumed delivery that does not continue the buffered one was
            // reassociated from a dissociated link endpoint
            self.on_reassociated_transfer(transfer, payload).await
        } else {
            // Final transfer of the delivery, including a resumed transfer
            // that continues the buffered delivery
            self.on_complete_transfer(transfer, payload).await // cancel safe
        }
    }

    /// Set the link credit. This will stop draining if the link is in a draining cycle
    ///
    /// # Cancel safety
    ///
    /// This is cancel safe as internanlly it only `.await` on sending over `tokio::mpsc::Sender`
    #[inline]
    pub async fn set_credit(&mut self, credit: SequenceNo) -> Result<(), LinkStateError> {
        self.processed.store(0, Ordering::Release);
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
                self.processed.store(0, Ordering::Release);
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
        self.processed.store(0, Ordering::Release);

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
    /// Switch the link to a new session, returning whether the link is being
    /// reattached (i.e. the new session is a different session from the
    /// current one).
    ///
    /// The new session may belong to a different connection whose negotiated
    /// max frame size differs, so the link's `max_frame_size` is refreshed
    /// from the new session before any attach/transfer frame is sent.
    pub(crate) fn switch_session<R>(&mut self, new_session: &SessionHandle<R>) {
        self.session = new_session.control.clone();
        self.outgoing = new_session.outgoing.clone();
        self.link.max_frame_size = new_session.max_frame_size();
    }

    pub(crate) async fn resume_incoming_attach(
        &mut self,
        mut initial_remote_attach: Option<Attach>,
    ) -> Result<ReceiverAttachExchange, ReceiverResumeErrorKind> {
        self.reallocate_output_handle().await?;

        let exchange = match initial_remote_attach.take() {
            Some(remote_attach) => {
                self.link.send_attach(&self.outgoing).await?;
                self.link.on_incoming_attach(remote_attach)?
            }
            None => self.exchange_attach().await?,
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
    inner: Box<ReceiverInner<ReceiverLink<Target>>>,
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

    async fn resume_inner(mut self) -> Result<ResumingReceiver, ReceiverResumeError> {
        let exchange = try_as_recver!(self, self.inner.resume_incoming_attach(None).await);
        let receiver = Receiver { inner: *self.inner };
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
        self.resume_inner().await
    }

    cfg_not_wasm32! {
        async fn resume_with_timeout_inner(
            mut self,
            duration: Duration,
        ) -> Result<ResumingReceiver, ReceiverResumeError> {
            let fut = self.inner.resume_incoming_attach(None);

            match tokio::time::timeout(duration, fut).await {
                Ok(Ok(exchange)) => {
                    let receiver = Receiver { inner: *self.inner };
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
            self.resume_with_timeout_inner(duration).await
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
        self.inner.switch_session(session);

        self.resume_inner().await
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
            self.inner.resume_incoming_attach(Some(remote_attach)).await
        );
        let receiver = Receiver { inner: *self.inner };
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
        self.inner.switch_session(session);

        let exchange = try_as_recver!(
            self,
            self.inner.resume_incoming_attach(Some(remote_attach)).await
        );
        let receiver = Receiver { inner: *self.inner };
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
            self.inner.switch_session(session);
            self.resume_with_timeout_inner(duration).await
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
            let fut = self.inner.resume_incoming_attach(Some(remote_attach));

            match tokio::time::timeout(duration, fut).await {
                Ok(Ok(exchange)) => {
                    let receiver = Receiver { inner: *self.inner };
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
            self.inner.switch_session(session);

            let fut = self.inner.resume_incoming_attach(Some(remote_attach));

            match tokio::time::timeout(duration, fut).await {
                Ok(Ok(exchange)) => {
                    let receiver = Receiver { inner: *self.inner };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::OutputHandle;
    use crate::link::state::{LinkFlowState, LinkFlowStateInner, LinkState};
    use crate::util::Sealed;
    use fe2o3_amqp_types::{
        definitions::SenderSettleMode, messaging::Received, primitives::OrderedMap,
    };
    use tokio::sync::oneshot;

    fn make_flow_state(link_credit: u32) -> ReceiverFlowState {
        Arc::new(LinkFlowState::receiver(LinkFlowStateInner {
            initial_delivery_count: 0,
            delivery_count: 0,
            link_credit,
            available: 0,
            drain: false,
            properties: None,
        }))
    }

    fn make_disposer(
        tx: mpsc::Sender<LinkFrame>,
        unsettled: ArcReceiverUnsettledMap,
        credit_mode: CreditMode,
    ) -> ReceiverDisposer {
        ReceiverDisposer {
            outgoing: tx,
            unsettled,
            rcv_settle_mode: ReceiverSettleMode::First,
            flow_state: make_flow_state(200),
            output_handle: Some(OutputHandle(1)),
            processed: Arc::new(AtomicU32::new(0)),
            credit_mode,
            session_stop_reason: Arc::new(OnceLock::new()),
        }
    }

    fn make_delivery_info(id: u32, tag: Vec<u8>) -> DeliveryInfo {
        DeliveryInfo {
            delivery_id: id,
            delivery_tag: DeliveryTag::from(tag),
            rcv_settle_mode: None,
            _sealed: Sealed {},
        }
    }

    fn seed_unsettled(map: &ArcReceiverUnsettledMap, tags: &[Vec<u8>]) {
        let mut lock = map.write();
        let m = lock.get_or_insert_with(Default::default);
        for tag in tags {
            m.insert(DeliveryTag::from(tag.clone()), None);
        }
    }

    #[tokio::test]
    async fn accept_sends_disposition_frame() {
        let (tx, mut rx) = mpsc::channel::<LinkFrame>(16);
        let unsettled: ArcReceiverUnsettledMap = Arc::new(parking_lot::RwLock::new(None));
        seed_unsettled(&unsettled, &[vec![0x01]]);

        let disposer = make_disposer(tx, unsettled, CreditMode::Manual);
        let info = make_delivery_info(1, vec![0x01]);

        disposer.accept(info).await.unwrap();

        let frame = rx.try_recv().expect("should receive a frame");
        match frame {
            LinkFrame::Disposition(d) => {
                assert_eq!(d.role, Role::Receiver);
                assert_eq!(d.first, 1);
                assert!(d.settled);
                assert!(matches!(d.state, Some(DeliveryState::Accepted(_))));
            }
            other => panic!("expected Disposition, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn release_sends_released_disposition() {
        let (tx, mut rx) = mpsc::channel::<LinkFrame>(16);
        let unsettled: ArcReceiverUnsettledMap = Arc::new(parking_lot::RwLock::new(None));
        seed_unsettled(&unsettled, &[vec![0x02]]);

        let disposer = make_disposer(tx, unsettled, CreditMode::Manual);
        let info = make_delivery_info(2, vec![0x02]);

        disposer.release(info).await.unwrap();

        let frame = rx.try_recv().expect("should receive a frame");
        match frame {
            LinkFrame::Disposition(d) => {
                assert_eq!(d.first, 2);
                assert!(d.settled);
                assert!(matches!(d.state, Some(DeliveryState::Released(_))));
            }
            other => panic!("expected Disposition, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn accept_removes_from_unsettled_map() {
        let (tx, _rx) = mpsc::channel::<LinkFrame>(16);
        let unsettled: ArcReceiverUnsettledMap = Arc::new(parking_lot::RwLock::new(None));
        seed_unsettled(&unsettled, &[vec![0x0A], vec![0x0B]]);

        let disposer = make_disposer(tx, unsettled.clone(), CreditMode::Manual);
        let info = make_delivery_info(10, vec![0x0A]);

        disposer.accept(info).await.unwrap();

        let lock = unsettled.read();
        let map = lock.as_ref().unwrap();
        assert!(!map.contains_key(&DeliveryTag::from(vec![0x0A])));
        assert!(map.contains_key(&DeliveryTag::from(vec![0x0B])));
    }

    #[tokio::test]
    async fn processed_counter_increments() {
        let (tx, _rx) = mpsc::channel::<LinkFrame>(16);
        let unsettled: ArcReceiverUnsettledMap = Arc::new(parking_lot::RwLock::new(None));
        seed_unsettled(&unsettled, &[vec![1], vec![2], vec![3]]);

        let disposer = make_disposer(tx, unsettled, CreditMode::Manual);

        disposer
            .accept(make_delivery_info(1, vec![1]))
            .await
            .unwrap();
        disposer
            .accept(make_delivery_info(2, vec![2]))
            .await
            .unwrap();
        disposer
            .accept(make_delivery_info(3, vec![3]))
            .await
            .unwrap();

        assert_eq!(disposer.processed.load(Ordering::Acquire), 3);
    }

    #[tokio::test]
    async fn auto_credit_refresh_sends_flow_frame() {
        // Auto(10) → triggers refresh after 5 dispositions
        let max_credit = 10u32;
        let (tx, mut rx) = mpsc::channel::<LinkFrame>(32);
        let unsettled: ArcReceiverUnsettledMap = Arc::new(parking_lot::RwLock::new(None));
        let tags: Vec<Vec<u8>> = (1..=5).map(|i| vec![i]).collect();
        seed_unsettled(&unsettled, &tags);

        let disposer = make_disposer(tx, unsettled, CreditMode::Auto(max_credit));

        for i in 1..=5u32 {
            disposer
                .accept(make_delivery_info(i, vec![i as u8]))
                .await
                .unwrap();
        }

        // Drain all frames: 5 Dispositions + 1 Flow
        let mut dispositions = 0;
        let mut flows = 0;
        while let Ok(frame) = rx.try_recv() {
            match frame {
                LinkFrame::Disposition(_) => dispositions += 1,
                LinkFrame::Flow(f) => {
                    flows += 1;
                    assert_eq!(f.link_credit, Some(max_credit));
                }
                other => panic!("unexpected frame: {other:?}"),
            }
        }
        assert_eq!(dispositions, 5);
        assert_eq!(flows, 1, "should send exactly one Flow to refresh credit");

        // processed should have been reset to 0 after the refresh
        assert_eq!(disposer.processed.load(Ordering::Acquire), 0);
    }

    #[tokio::test]
    async fn no_disposition_for_unknown_delivery_tag() {
        let (tx, mut rx) = mpsc::channel::<LinkFrame>(16);
        let unsettled: ArcReceiverUnsettledMap = Arc::new(parking_lot::RwLock::new(None));
        seed_unsettled(&unsettled, &[vec![0x01]]);

        let disposer = make_disposer(tx, unsettled, CreditMode::Manual);

        // Accept a tag that is NOT in the unsettled map
        let info = make_delivery_info(99, vec![0xFF]);
        disposer.accept(info).await.unwrap();

        // No Disposition frame should be sent
        assert!(rx.try_recv().is_err());
        // But processed still incremented
        assert_eq!(disposer.processed.load(Ordering::Acquire), 1);
    }

    #[tokio::test]
    async fn clone_shares_state() {
        let (tx, _rx) = mpsc::channel::<LinkFrame>(16);
        let unsettled: ArcReceiverUnsettledMap = Arc::new(parking_lot::RwLock::new(None));
        seed_unsettled(&unsettled, &[vec![1], vec![2]]);

        let disposer = make_disposer(tx, unsettled, CreditMode::Manual);
        let clone = disposer.clone();

        // Accept via the clone
        clone.accept(make_delivery_info(1, vec![1])).await.unwrap();

        // Original sees the incremented counter
        assert_eq!(disposer.processed.load(Ordering::Acquire), 1);

        // Unsettled map updated via clone is visible from original
        let lock = disposer.unsettled.read();
        let map = lock.as_ref().unwrap();
        assert!(!map.contains_key(&DeliveryTag::from(vec![1u8])));
        assert!(map.contains_key(&DeliveryTag::from(vec![2u8])));
    }

    #[tokio::test]
    async fn concurrent_accepts_are_safe() {
        let (tx, mut rx) = mpsc::channel::<LinkFrame>(1024);
        let unsettled: ArcReceiverUnsettledMap = Arc::new(parking_lot::RwLock::new(None));
        let tags: Vec<Vec<u8>> = (0..100u8).map(|i| vec![i]).collect();
        seed_unsettled(&unsettled, &tags);

        let disposer = make_disposer(tx, unsettled, CreditMode::Manual);

        // Spawn 100 concurrent accepts
        let handles: Vec<_> = (0..100u8)
            .map(|i| {
                let d = disposer.clone();
                tokio::spawn(async move {
                    d.accept(make_delivery_info(i as u32, vec![i]))
                        .await
                        .unwrap();
                })
            })
            .collect();

        futures_util::future::join_all(handles).await;

        assert_eq!(disposer.processed.load(Ordering::Acquire), 100);

        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 100);
    }

    fn make_receiver_inner_with_channels(
        max_frame_size: usize,
    ) -> (
        ReceiverInner<ReceiverLink<Target>>,
        mpsc::Receiver<SessionControl>,
        mpsc::Receiver<LinkFrame>,
        mpsc::Sender<LinkFrame>,
    ) {
        let (session_tx, session_rx) = mpsc::channel::<SessionControl>(16);
        let (outgoing_tx, outgoing_rx) = mpsc::channel::<LinkFrame>(16);
        let (incoming_tx, incoming_rx) = mpsc::channel::<LinkFrame>(16);
        let unsettled: ArcReceiverUnsettledMap = Arc::new(parking_lot::RwLock::new(None));
        let link = ReceiverLink::<Target> {
            role: std::marker::PhantomData,
            local_state: LinkState::Attached,
            name: String::from("test-receiver"),
            output_handle: Some(OutputHandle(0)),
            input_handle: None,
            snd_settle_mode: SenderSettleMode::Mixed,
            rcv_settle_mode: ReceiverSettleMode::First,
            source: None,
            target: None,
            max_message_size: 0,
            offered_capabilities: None,
            desired_capabilities: None,
            flow_state: make_flow_state(200),
            unsettled,
            session_stop_reason: Arc::new(OnceLock::new()),
            max_frame_size,
            verify_incoming_source: true,
            verify_incoming_target: true,
        };
        let inner = ReceiverInner {
            link,
            buffer_size: 16,
            credit_mode: CreditMode::Auto(200),
            processed: Arc::new(AtomicU32::new(0)),
            auto_accept: false,
            session: session_tx,
            outgoing: outgoing_tx,
            incoming: incoming_rx,
            incomplete_transfer: None,
        };
        (inner, session_rx, outgoing_rx, incoming_tx)
    }

    fn make_receiver_inner(max_frame_size: usize) -> ReceiverInner<ReceiverLink<Target>> {
        make_receiver_inner_with_channels(max_frame_size).0
    }

    fn make_incoming_transfer(
        delivery_id: u32,
        delivery_tag: Option<Vec<u8>>,
        more: bool,
        settled: bool,
    ) -> Transfer {
        Transfer {
            handle: Handle(0),
            delivery_id: Some(delivery_id),
            delivery_tag: delivery_tag.map(DeliveryTag::from),
            message_format: Some(0),
            settled: Some(settled),
            more,
            rcv_settle_mode: None,
            state: None,
            resume: false,
            aborted: false,
            batchable: false,
        }
    }

    fn make_link_frame(transfer: Transfer, payload: Payload) -> LinkFrame {
        LinkFrame::Transfer {
            input_handle: crate::endpoint::InputHandle(0),
            performative: transfer,
            payload,
        }
    }

    fn encoded_message_payload(body: &str) -> Payload {
        use fe2o3_amqp_types::messaging::{message::__private::Serializable, Message};
        let message = Message::from(body.to_string());
        Payload::from(serde_amqp::to_vec(&Serializable(message)).unwrap())
    }

    /// An encoded message with a header and a body section, so that section
    /// numbering can locate the body (`section_number == 2`).
    fn encoded_sectioned_payload() -> Payload {
        use fe2o3_amqp_types::{
            messaging::{message::__private::Serializable, AmqpValue, Body, Header, Message},
            primitives::Value,
        };
        let message = Message {
            header: Some(Header {
                durable: true,
                ..Default::default()
            }),
            delivery_annotations: None,
            message_annotations: None,
            properties: None,
            application_properties: None,
            body: Body::Value(AmqpValue(Value::Bool(true))),
            footer: None,
        };
        Payload::from(serde_amqp::to_vec(&Serializable(message)).unwrap())
    }

    fn assert_message_size_exceeded(error: RecvError, expected_size: u64, expected_max: u64) {
        match error {
            RecvError::MessageSizeExceeded(e) => {
                assert_eq!(e.size, expected_size);
                assert_eq!(e.max_size, expected_max);
            }
            other => panic!("expected MessageSizeExceeded, got {other:?}"),
        }
    }

    /// Drive the closing-detach handshake while `recv()` processes a malformed
    /// or oversized transfer: assert that the detach closes the link with the
    /// expected condition, answer it, and return the error `recv()` produced.
    async fn recv_expecting_fatal_close(
        inner: &mut ReceiverInner<ReceiverLink<Target>>,
        outgoing_rx: &mut mpsc::Receiver<LinkFrame>,
        incoming_tx: &mpsc::Sender<LinkFrame>,
        expected_condition: definitions::ErrorCondition,
    ) -> RecvError {
        let (result, ()) = tokio::join!(inner.recv::<String>(), async {
            match outgoing_rx.recv().await.expect("expected a closing detach") {
                LinkFrame::Detach(detach) => {
                    assert!(detach.closed, "the error detach must close the link");
                    let error = detach
                        .error
                        .as_ref()
                        .expect("the detach must carry the error");
                    assert_eq!(error.condition, expected_condition);
                }
                other => panic!("expected Detach, got {other:?}"),
            }

            // Answer the closing detach to complete the handshake.
            incoming_tx
                .send(LinkFrame::Detach(Detach {
                    handle: Handle(0),
                    closed: true,
                    error: None,
                }))
                .await
                .unwrap();
        });

        result.expect_err("a malformed delivery must fail")
    }

    /// The minimal `Attach` a sender peer sends for this receiver link: it
    /// must carry a source and an initial delivery count so
    /// `on_incoming_attach` accepts it.
    fn peer_sender_attach() -> Attach {
        Attach {
            name: String::from("test-receiver"),
            handle: Handle(0),
            role: Role::Sender,
            snd_settle_mode: SenderSettleMode::Mixed,
            rcv_settle_mode: ReceiverSettleMode::First,
            source: Some(Box::new(Source::default())),
            target: None,
            unsettled: None,
            incomplete_unsettled: false,
            initial_delivery_count: Some(0),
            max_message_size: None,
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,
        }
    }

    fn make_session_handle(max_frame_size: usize) -> SessionHandle<()> {
        let (control, _control_rx) = mpsc::channel::<SessionControl>(16);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel::<LinkFrame>(16);
        let (outcome_tx, outcome) = oneshot::channel::<Result<(), crate::session::error::Error>>();
        drop(outcome_tx);
        SessionHandle {
            is_ended: false,
            control,
            engine_handle: tokio::spawn(async {}),
            outcome,
            outgoing: outgoing_tx,
            session_stop_reason: Arc::new(OnceLock::new()),
            max_frame_size,
            link_listener: (),
        }
    }

    #[tokio::test]
    async fn switch_session_refreshes_max_frame_size() {
        let mut inner = make_receiver_inner(4092);
        let session_b = make_session_handle(1020);

        // Switching to a different session refreshes the link's max frame
        // size from the new session
        inner.switch_session(&session_b);
        assert_eq!(inner.link.max_frame_size, 1020);

        // Switching to the same session is idempotent
        inner.switch_session(&session_b);
        assert_eq!(inner.link.max_frame_size, 1020);
    }

    /// The accumulated size must count the buffered chunks for a tagless
    /// continuation (AMQP 1.0 §2.7.5) while an explicit tag of another
    /// delivery does not.
    #[tokio::test]
    async fn accumulated_size_counts_tagless_continuation_but_not_other_delivery() {
        let mut inner = make_receiver_inner(4096);
        inner
            .on_incoming_transfer::<String>(
                make_incoming_transfer(1, Some(vec![0x01]), true, false),
                Payload::from(vec![0u8; 60]),
            )
            .await
            .unwrap();

        let continuation = make_incoming_transfer(1, None, true, false);
        assert_eq!(inner.accumulated_message_size(&continuation), 60);

        let other_delivery = make_incoming_transfer(2, Some(vec![0x02]), true, false);
        assert_eq!(inner.accumulated_message_size(&other_delivery), 0);
    }

    /// A delivery spanning exactly two frames whose total size exceeds the
    /// link's max-message-size must detach the link on the tagless final frame
    /// (AMQP 1.0 §2.6.5, `amqp:link:message-size-exceeded`).
    #[tokio::test]
    async fn two_frame_delivery_exceeding_max_message_size_is_rejected_on_final_frame() {
        let (mut inner, _session_rx, mut outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);
        inner.link.max_message_size = 100;

        // The first frame carries the delivery tag, the final continuation
        // omits it.
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, Some(vec![0x01]), true, false),
                Payload::from(vec![0u8; 60]),
            ))
            .await
            .unwrap();
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, None, false, false),
                Payload::from(vec![0u8; 60]),
            ))
            .await
            .unwrap();

        // An oversized message is a link error: the link is detached with
        // `amqp:link:message-size-exceeded` and destroyed.
        let error = recv_expecting_fatal_close(
            &mut inner,
            &mut outgoing_rx,
            &incoming_tx,
            definitions::ErrorCondition::from(LinkError::MessageSizeExceeded),
        )
        .await;
        assert_message_size_exceeded(error, 120, 100);
        assert!(inner.incomplete_transfer.is_none());
        assert!(matches!(inner.link.local_state, LinkState::Closed));
    }

    /// A delivery spanning three or more frames must detach the link as soon
    /// as an intermediate `more=true` frame pushes the accumulated size over
    /// the limit, before the final frame arrives.
    #[tokio::test]
    async fn multi_frame_delivery_exceeding_max_message_size_is_rejected_on_intermediate_frame() {
        let (mut inner, _session_rx, mut outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);
        inner.link.max_message_size = 100;

        // 50 + 50 reaches the limit but does not exceed it.
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, Some(vec![0x01]), true, false),
                Payload::from(vec![0u8; 50]),
            ))
            .await
            .unwrap();
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, None, true, false),
                Payload::from(vec![0u8; 50]),
            ))
            .await
            .unwrap();
        // The next intermediate frame exceeds the limit.
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, None, true, false),
                Payload::from(vec![0u8; 1]),
            ))
            .await
            .unwrap();

        let error = recv_expecting_fatal_close(
            &mut inner,
            &mut outgoing_rx,
            &incoming_tx,
            definitions::ErrorCondition::from(LinkError::MessageSizeExceeded),
        )
        .await;
        assert_message_size_exceeded(error, 101, 100);
        assert!(inner.incomplete_transfer.is_none());
        assert!(matches!(inner.link.local_state, LinkState::Closed));
    }

    /// A tagless-continuation delivery whose total size is exactly the limit
    /// is accepted and decoded.
    #[tokio::test]
    async fn in_limit_multi_frame_delivery_is_delivered() {
        let (mut inner, _session_rx, _outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);

        let body = "in-limit multi-frame message";
        let payload = encoded_message_payload(body);
        let total = payload.len() as u64;
        // Exactly at the limit: the enforcement uses `>` and must not reject.
        inner.link.max_message_size = total;

        let split = payload.len() / 2;
        let first = payload.slice(..split);
        let second = payload.slice(split..);
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, Some(vec![0x01]), true, false),
                first,
            ))
            .await
            .unwrap();
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, None, false, false),
                second,
            ))
            .await
            .unwrap();

        let delivery = inner
            .recv::<String>()
            .await
            .expect("in-limit delivery must be accepted");
        assert_eq!(delivery.body(), body);
    }

    /// The first transfer of a multi-transfer delivery must carry the
    /// delivery-id and delivery-tag (AMQP 1.0 §2.7.5); a violation closes the
    /// link with `amqp:not-allowed` (AMQP 1.0 §2.6.5).
    #[tokio::test]
    async fn first_multi_frame_transfer_missing_mandatory_fields_closes_link() {
        // Missing delivery-tag
        let (mut inner, _session_rx, mut outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, None, true, false),
                Payload::from(vec![0u8; 8]),
            ))
            .await
            .unwrap();
        let error = recv_expecting_fatal_close(
            &mut inner,
            &mut outgoing_rx,
            &incoming_tx,
            definitions::ErrorCondition::from(definitions::AmqpError::NotAllowed),
        )
        .await;
        assert!(matches!(error, RecvError::DeliveryTagIsNone));
        assert!(matches!(inner.link.local_state, LinkState::Closed));

        // Missing delivery-id
        let (mut inner, _session_rx, mut outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);
        let mut missing_id = make_incoming_transfer(1, Some(vec![0x01]), true, false);
        missing_id.delivery_id = None;
        incoming_tx
            .send(make_link_frame(missing_id, Payload::from(vec![0u8; 8])))
            .await
            .unwrap();
        let error = recv_expecting_fatal_close(
            &mut inner,
            &mut outgoing_rx,
            &incoming_tx,
            definitions::ErrorCondition::from(definitions::AmqpError::NotAllowed),
        )
        .await;
        assert!(matches!(error, RecvError::DeliveryIdIsNone));
        assert!(matches!(inner.link.local_state, LinkState::Closed));
    }

    /// The mandatory delivery-id and delivery-tag are required on the first
    /// transfer of any delivery, including a single-frame one (stricter than
    /// the transfer field text, matching go-amqp and Qpid Proton).
    #[tokio::test]
    async fn single_frame_transfer_missing_mandatory_fields_closes_link() {
        // Missing delivery-tag
        let (mut inner, _session_rx, mut outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, None, false, false),
                Payload::from(vec![0u8; 8]),
            ))
            .await
            .unwrap();
        let error = recv_expecting_fatal_close(
            &mut inner,
            &mut outgoing_rx,
            &incoming_tx,
            definitions::ErrorCondition::from(definitions::AmqpError::NotAllowed),
        )
        .await;
        assert!(matches!(error, RecvError::DeliveryTagIsNone));
        assert!(matches!(inner.link.local_state, LinkState::Closed));

        // Missing delivery-id
        let (mut inner, _session_rx, mut outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);
        let mut missing_id = make_incoming_transfer(1, Some(vec![0x01]), false, false);
        missing_id.delivery_id = None;
        incoming_tx
            .send(make_link_frame(missing_id, Payload::from(vec![0u8; 8])))
            .await
            .unwrap();
        let error = recv_expecting_fatal_close(
            &mut inner,
            &mut outgoing_rx,
            &incoming_tx,
            definitions::ErrorCondition::from(definitions::AmqpError::NotAllowed),
        )
        .await;
        assert!(matches!(error, RecvError::DeliveryIdIsNone));
        assert!(matches!(inner.link.local_state, LinkState::Closed));
    }

    /// A state-carrying transfer with no buffered delivery and no delivery-tag
    /// cannot be attributed to a delivery and closes the link.
    #[tokio::test]
    async fn tagless_state_transfer_without_buffer_closes_link() {
        let (mut inner, _session_rx, mut outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);

        let mut state_transfer = make_incoming_transfer(1, None, false, false);
        state_transfer.state = Some(DeliveryState::Received(Received {
            section_number: 0,
            section_offset: 0,
        }));
        incoming_tx
            .send(make_link_frame(state_transfer, Payload::new()))
            .await
            .unwrap();

        let error = recv_expecting_fatal_close(
            &mut inner,
            &mut outgoing_rx,
            &incoming_tx,
            definitions::ErrorCondition::from(definitions::AmqpError::NotAllowed),
        )
        .await;
        assert!(matches!(error, RecvError::DeliveryTagIsNone));
        assert!(matches!(inner.link.local_state, LinkState::Closed));
    }

    /// A continuation that explicitly names a different delivery while one is
    /// being assembled violates AMQP 1.0 §2.6.14 and closes the link.
    #[tokio::test]
    async fn continuation_with_different_delivery_tag_closes_link() {
        let (mut inner, _session_rx, mut outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);

        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, Some(vec![0x01]), true, false),
                Payload::from(vec![0u8; 8]),
            ))
            .await
            .unwrap();
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, Some(vec![0x02]), false, false),
                Payload::from(vec![0u8; 8]),
            ))
            .await
            .unwrap();

        let error = recv_expecting_fatal_close(
            &mut inner,
            &mut outgoing_rx,
            &incoming_tx,
            definitions::ErrorCondition::from(definitions::AmqpError::NotAllowed),
        )
        .await;
        assert!(matches!(
            error,
            RecvError::InconsistentFieldInMultiFrameDelivery
        ));
        assert!(matches!(inner.link.local_state, LinkState::Closed));
    }

    /// A continuation whose present message-format differs from the first
    /// transfer is rejected (AMQP 1.0 §2.7.5) and closes the link.
    #[tokio::test]
    async fn continuation_with_different_message_format_closes_link() {
        let (mut inner, _session_rx, mut outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);

        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, Some(vec![0x01]), true, false),
                Payload::from(vec![0u8; 8]),
            ))
            .await
            .unwrap();

        let mut different_format = make_incoming_transfer(1, Some(vec![0x01]), false, false);
        different_format.message_format = Some(1);
        incoming_tx
            .send(make_link_frame(
                different_format,
                Payload::from(vec![0u8; 8]),
            ))
            .await
            .unwrap();

        let error = recv_expecting_fatal_close(
            &mut inner,
            &mut outgoing_rx,
            &incoming_tx,
            definitions::ErrorCondition::from(definitions::AmqpError::NotAllowed),
        )
        .await;
        assert!(matches!(
            error,
            RecvError::InconsistentFieldInMultiFrameDelivery
        ));
        assert!(matches!(inner.link.local_state, LinkState::Closed));
    }

    /// An aborted transfer matching the buffered delivery discards its chunks
    /// and its entry in the unsettled map; the link stays usable.
    #[tokio::test]
    async fn aborted_transfer_discards_the_buffered_delivery() {
        let (mut inner, _session_rx, _outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);
        let aborted_tag = DeliveryTag::from(vec![0x01]);

        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, Some(vec![0x01]), true, false),
                Payload::from(vec![0u8; 8]),
            ))
            .await
            .unwrap();

        // The sender aborts the delivery (`resume` mirrors `Sender::abort`).
        let mut abort = make_incoming_transfer(1, Some(vec![0x01]), false, false);
        abort.resume = true;
        abort.aborted = true;
        incoming_tx
            .send(make_link_frame(abort, Payload::new()))
            .await
            .unwrap();

        // The link remains usable: a following ordinary message is delivered.
        let body = "after abort";
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(2, Some(vec![0x02]), false, false),
                encoded_message_payload(body),
            ))
            .await
            .unwrap();

        let delivery = inner.recv::<String>().await.expect("ordinary delivery");
        assert_eq!(delivery.body(), body);
        assert!(inner.incomplete_transfer.is_none());

        let guard = inner.link.unsettled().read();
        assert!(guard
            .as_ref()
            .is_none_or(|map| !map.contains_key(&aborted_tag)));
    }

    /// The same as above when the aborting transfer omits the delivery tag:
    /// the buffered delivery's tag identifies the entry to discard.
    #[tokio::test]
    async fn aborted_transfer_with_omitted_tag_discards_the_buffered_delivery() {
        let (mut inner, _session_rx, _outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);
        let aborted_tag = DeliveryTag::from(vec![0x01]);

        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, Some(vec![0x01]), true, false),
                Payload::from(vec![0u8; 8]),
            ))
            .await
            .unwrap();

        let mut abort = make_incoming_transfer(1, None, false, false);
        abort.resume = true;
        abort.aborted = true;
        incoming_tx
            .send(make_link_frame(abort, Payload::new()))
            .await
            .unwrap();

        let body = "after abort";
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(2, Some(vec![0x02]), false, false),
                encoded_message_payload(body),
            ))
            .await
            .unwrap();

        let delivery = inner.recv::<String>().await.expect("ordinary delivery");
        assert_eq!(delivery.body(), body);
        assert!(inner.incomplete_transfer.is_none());

        let guard = inner.link.unsettled().read();
        assert!(guard
            .as_ref()
            .is_none_or(|map| !map.contains_key(&aborted_tag)));
    }

    /// An aborted transfer with no buffered delivery is ignored.
    #[tokio::test]
    async fn aborted_transfer_without_buffer_is_ignored() {
        let (mut inner, _session_rx, _outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);

        let mut abort = make_incoming_transfer(1, Some(vec![0x09]), false, false);
        abort.resume = true;
        abort.aborted = true;
        incoming_tx
            .send(make_link_frame(abort, Payload::from(vec![0u8; 4])))
            .await
            .unwrap();

        let body = "after ignored abort";
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(2, Some(vec![0x0A]), false, false),
                encoded_message_payload(body),
            ))
            .await
            .unwrap();

        let delivery = inner.recv::<String>().await.expect("ordinary delivery");
        assert_eq!(delivery.body(), body);
        assert!(inner.incomplete_transfer.is_none());
    }

    /// An aborted transfer that explicitly names another delivery while one
    /// is being assembled violates AMQP 1.0 §2.6.14 and closes the link.
    #[tokio::test]
    async fn aborted_transfer_with_different_delivery_tag_closes_link() {
        let (mut inner, _session_rx, mut outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);

        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(1, Some(vec![0x01]), true, false),
                Payload::from(vec![0u8; 8]),
            ))
            .await
            .unwrap();

        let mut abort = make_incoming_transfer(1, Some(vec![0x02]), false, false);
        abort.resume = true;
        abort.aborted = true;
        incoming_tx
            .send(make_link_frame(abort, Payload::new()))
            .await
            .unwrap();

        let error = recv_expecting_fatal_close(
            &mut inner,
            &mut outgoing_rx,
            &incoming_tx,
            definitions::ErrorCondition::from(definitions::AmqpError::NotAllowed),
        )
        .await;
        assert!(matches!(
            error,
            RecvError::InconsistentFieldInMultiFrameDelivery
        ));
        assert!(matches!(inner.link.local_state, LinkState::Closed));
    }

    /// AMQP 1.0 §2.6.13: a resumed delivery that is not in the local
    /// unsettled map is ignored; the link stays usable.
    #[tokio::test]
    async fn resumed_delivery_unknown_to_unsettled_map_is_ignored() {
        let (mut inner, _session_rx, _outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);

        let ignored_tag = DeliveryTag::from(vec![0x0A]);
        let mut resumed = make_incoming_transfer(1, Some(vec![0x0A]), false, false);
        resumed.resume = true;
        incoming_tx
            .send(make_link_frame(resumed, encoded_message_payload("ignored")))
            .await
            .unwrap();

        // A following ordinary message is delivered; the ignored one is not.
        let body = "ordinary";
        incoming_tx
            .send(make_link_frame(
                make_incoming_transfer(2, Some(vec![0x0B]), false, false),
                encoded_message_payload(body),
            ))
            .await
            .unwrap();

        let delivery = inner.recv::<String>().await.expect("ordinary delivery");
        assert_eq!(delivery.body(), body);

        let guard = inner.link.unsettled().read();
        assert!(guard
            .as_ref()
            .is_none_or(|map| !map.contains_key(&ignored_tag)));
    }

    /// A resumed delivery present in the local unsettled map is delivered.
    #[tokio::test]
    async fn resumed_delivery_known_to_unsettled_map_is_delivered() {
        let (mut inner, _session_rx, _outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);

        seed_unsettled(&inner.link.unsettled, &[vec![0x0A]]);

        let body = "resumed";
        let mut resumed = make_incoming_transfer(1, Some(vec![0x0A]), false, false);
        resumed.resume = true;
        incoming_tx
            .send(make_link_frame(resumed, encoded_message_payload(body)))
            .await
            .unwrap();

        let delivery = inner.recv::<String>().await.expect("resumed delivery");
        assert_eq!(delivery.body(), body);
    }

    /// A tagless continuation carrying a `Received` state is attributed to the
    /// buffered delivery instead of failing with `DeliveryTagIsNone`, and it
    /// trims the buffer to the reported section and offset.
    #[tokio::test]
    async fn tagless_received_state_transfer_trims_the_buffer() {
        let mut inner = make_receiver_inner(4096);
        let tag = DeliveryTag::from(vec![0x01]);

        let payload = encoded_sectioned_payload();
        let full_len = payload.len() as u64;
        inner
            .on_incoming_transfer::<String>(
                make_incoming_transfer(1, Some(vec![0x01]), true, false),
                payload,
            )
            .await
            .unwrap();

        let mut state_transfer = make_incoming_transfer(1, None, true, false);
        state_transfer.state = Some(DeliveryState::Received(Received {
            section_number: 2,
            section_offset: 0,
        }));
        inner
            .on_incoming_transfer::<String>(state_transfer, Payload::new())
            .await
            .expect("a tagless state transfer must be attributed to the buffer");

        // The buffer was trimmed to the reported section.
        let incomplete = inner
            .incomplete_transfer
            .as_ref()
            .expect("the delivery is still being assembled");
        assert!(
            incomplete.accumulated_payload_size() < full_len,
            "the Received state must trim the buffer"
        );

        let guard = inner.link.unsettled().read();
        let state = guard
            .as_ref()
            .and_then(|map| map.get(&tag))
            .cloned()
            .flatten();
        assert!(matches!(
            state,
            Some(DeliveryState::Received(Received {
                section_number: 2,
                ..
            }))
        ));
    }

    /// AMQP 1.0 §2.6.13: target-only deliveries (local unsettled entries
    /// absent from a complete remote map) MUST be considered settled.
    #[test]
    fn receiver_reconciles_target_only_deliveries() {
        let (mut inner, _session_rx, _outgoing_rx, _incoming_tx) =
            make_receiver_inner_with_channels(4096);
        seed_unsettled(&inner.link.unsettled, &[vec![1], vec![2]]);

        // Remote map has only tag 1 (complete): tag 2 is target-only.
        let mut remote = OrderedMap::new();
        remote.insert(DeliveryTag::from(vec![1u8]), None);
        let _ = inner.link.handle_unsettled_in_attach(Some(remote), false);

        let lock = inner.link.unsettled.read();
        let map = lock.as_ref().unwrap();
        assert!(map.contains_key(&DeliveryTag::from(vec![1u8])));
        assert!(!map.contains_key(&DeliveryTag::from(vec![2u8])));
    }

    /// An incomplete remote map is not evidence of settlement, so local
    /// entries are left untouched.
    #[test]
    fn receiver_keeps_deliveries_on_incomplete_remote_map() {
        let (mut inner, _session_rx, _outgoing_rx, _incoming_tx) =
            make_receiver_inner_with_channels(4096);
        seed_unsettled(&inner.link.unsettled, &[vec![1]]);

        let _ = inner
            .link
            .handle_unsettled_in_attach(Some(OrderedMap::new()), true);

        let lock = inner.link.unsettled.read();
        assert!(lock
            .as_ref()
            .unwrap()
            .contains_key(&DeliveryTag::from(vec![1u8])));
    }

    /// A `None` remote map (peer reattached without unsettled state) settles
    /// every target-only delivery.
    #[test]
    fn receiver_clears_deliveries_on_missing_remote_map() {
        let (mut inner, _session_rx, _outgoing_rx, _incoming_tx) =
            make_receiver_inner_with_channels(4096);
        seed_unsettled(&inner.link.unsettled, &[vec![1], vec![2]]);

        let _ = inner.link.handle_unsettled_in_attach(None, false);

        let lock = inner.link.unsettled.read();
        assert!(lock.as_ref().unwrap().is_empty());
    }

    /// A peer that suspends (non-closing detach) while this side is closing
    /// triggers the AMQP 1.0 §2.6.6 simultaneous-detach handshake.
    ///
    /// The spec assigns the reattach to the non-closing (suspending) side and
    /// only requires the closing side to complete the exchange. This
    /// implementation drives the reattach from both sides:
    ///
    /// - sending our closing detach releases the link from the session
    ///   (`Session::on_outgoing_detach`), so the link must be reattached
    ///   (`reattach_then_close` -> `reallocate_output_handle` ->
    ///   `allocate_link`) to re-register it; otherwise the peer's crossed
    ///   `Attach`/`Detach` could not be routed to the link and would end the
    ///   session;
    /// - with both sides reattaching, each side's attach exchange accepts the
    ///   peer's `Attach` as its answer, so the crossed detaches converge
    ///   symmetrically without depending on whether the peer drives its
    ///   reattach.
    ///
    /// The peer's frames are scripted, so this is deterministic.
    #[tokio::test]
    async fn close_reattaches_and_closes_on_simultaneous_suspend() {
        let (mut inner, session_rx, outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);

        let (result, (saw_attach, detaches)) = tokio::join!(
            inner.close_with_error(None),
            crate::link::test_util::drive_simultaneous_detach_race(
                session_rx,
                outgoing_rx,
                incoming_tx,
                peer_sender_attach(),
            ),
        );

        assert!(saw_attach, "the closing side must reattach");
        assert_eq!(
            detaches, 2,
            "expected a detach before and after the reattach"
        );
        assert!(result.is_ok(), "close must complete: {result:?}");
        assert!(matches!(&inner.link.local_state, LinkState::Closed));
    }

    /// A peer that closes (closing detach) while this side is suspending
    /// triggers the AMQP 1.0 §2.6.6 simultaneous-detach handshake. The spec
    /// assigns the reattach to this side (the non-closing/suspending side),
    /// which reattaches and then sends a closing detach; the link ends
    /// `Closed`, reported as a `LinkOutcome::Closed` outcome.
    ///
    /// The peer's frames are scripted, so this is deterministic.
    #[tokio::test]
    async fn detach_reattaches_and_closes_on_simultaneous_close() {
        let (mut inner, session_rx, outgoing_rx, incoming_tx) =
            make_receiver_inner_with_channels(4096);

        let (result, (saw_attach, detaches)) = tokio::join!(
            inner.detach_with_error(None),
            crate::link::test_util::drive_simultaneous_detach_race(
                session_rx,
                outgoing_rx,
                incoming_tx,
                peer_sender_attach(),
            ),
        );

        assert!(saw_attach, "the suspending side must reattach");
        assert_eq!(
            detaches, 2,
            "expected a detach before and after the reattach"
        );
        assert!(matches!(
            result,
            Ok(LinkOutcome::Closed { remote_error: None })
        ));
        assert!(matches!(&inner.link.local_state, LinkState::Closed));
    }
}
