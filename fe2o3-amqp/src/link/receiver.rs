//! Implementation of AMQP1.0 receiver

use std::time::Duration;

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{self, SequenceNo},
    messaging::{
        message::DecodeIntoMessage, Accepted, Address, DeliveryState, Modified, Rejected, Released,
        Source, Target,
    },
    performatives::{Attach, Detach, Transfer},
};
use tokio::{
    sync::mpsc,
    time::{error::Elapsed, timeout},
};
use tracing::instrument;

use crate::{
    control::SessionControl,
    endpoint::{self, LinkAttach, LinkDetach, LinkExt},
    session::SessionHandle,
    util::DeliveryInfo,
    Payload,
};

use super::{
    builder::{self, WithTarget, WithoutName, WithoutSource},
    delivery::Delivery,
    error::DetachError,
    receiver_link::count_number_of_sections_and_offset,
    role,
    shared_inner::{LinkEndpointInner, LinkEndpointInnerDetach, LinkEndpointInnerReattach},
    ArcReceiverUnsettledMap, DispositionError, IllegalLinkStateError, LinkFrame, LinkRelay,
    LinkStateError, ReceiverAttachError, ReceiverAttachExchange, ReceiverFlowState, ReceiverLink,
    ReceiverResumeError, ReceiverResumeErrorKind, ReceiverTransferError, RecvError, DEFAULT_CREDIT,
};

#[cfg(feature = "transaction")]
use fe2o3_amqp_types::definitions::AmqpError;

macro_rules! or_assign {
    ($self:ident, $other:ident, $field:ident) => {
        match &$self.performative.$field {
            Some(value) => {
                if let Some(other_value) = $other.$field {
                    if *value != other_value {
                        return Err(ReceiverTransferError::InconsistentFieldInMultiFrameDelivery)
                    }
                }
            },
            None => {
                $self.performative.$field = $other.$field;
            }
        }
    };

    ($self:ident, $other:ident, $($field:ident), *) => {
        $(or_assign!($self, $other, $field);)*
    }
}

#[derive(Debug)]
pub(crate) struct IncompleteTransfer {
    pub performative: Transfer,
    pub buffer: Vec<Payload>,
    pub section_number: Option<u32>,
    pub section_offset: u64,
}

impl IncompleteTransfer {
    pub fn new(transfer: Transfer, partial_payload: Payload) -> Self {
        let (number, offset) = count_number_of_sections_and_offset(&partial_payload);
        // let mut buffer = BytesMut::new();
        // // TODO: anyway to make this not copying the bytes?
        // buffer.extend(partial_payload);
        Self {
            performative: transfer,
            buffer: vec![partial_payload],
            section_number: Some(number),
            section_offset: offset,
        }
    }

    /// Like `|=` operator but works on the field level
    pub fn or_assign(&mut self, other: Transfer) -> Result<(), ReceiverTransferError> {
        or_assign! {
            self, other,
            delivery_id,
            delivery_tag,
            message_format
        };

        // If not set on the first (or only) transfer for a (multi-transfer)
        // delivery, then the settled flag MUST be interpreted as being false. For
        // subsequent transfers in a multi-transfer delivery if the settled flag
        // is left unset then it MUST be interpreted as true if and only if the
        // value of the settled flag on any of the preceding transfers was true;
        // if no preceding transfer was sent with settled being true then the
        // value when unset MUST be taken as false.
        match &self.performative.settled {
            Some(value) => {
                if let Some(other_value) = other.settled {
                    if !value {
                        self.performative.settled = Some(other_value);
                    }
                }
            }
            None => self.performative.settled = other.settled,
        }

        if let Some(other_state) = other.state {
            if let Some(state) = &self.performative.state {
                // Note that if the transfer performative (or an earlier disposition
                // performative referring to the delivery) indicates that the delivery has
                // attained a terminal state, then no future transfer or disposition sent
                // by the sender can alter that terminal state.
                if !state.is_terminal() {
                    self.performative.state = Some(other_state);
                }
            } else {
                self.performative.state = Some(other_state);
            }
        }

        Ok(())
    }

    /// Append to the buffered payload
    pub fn append(&mut self, other: Payload) {
        // TODO: append section number and re-count section-offset
        // Count section numbers
        let (number, offset) = count_number_of_sections_and_offset(&other);
        match (&mut self.section_number, number) {
            (_, 0) => self.section_offset += offset,
            (None, 1) => {
                // The first section
                self.section_number = Some(0);
                self.section_offset = offset;
            }
            (None, _) => {
                self.section_number = Some(number - 1);
                self.section_offset = offset;
            }
            (Some(val), _) => {
                *val += number;
                self.section_offset = offset;
            }
        }

        self.buffer.push(other);
    }
}

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
/// let mut receiver = ReceiverInner::attach(
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
/// let mut receiver = ReceiverInner::builder()
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
    /// Creates a builder for the [`ReceiverInner`]
    pub fn builder(
    ) -> builder::Builder<role::Receiver, Target, WithoutName, WithoutSource, WithTarget> {
        builder::Builder::<role::Receiver, Target, _, _, _>::new()
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

    /// Get a reference to the link's target field
    pub fn target(&self) -> &Option<Target> {
        &self.inner.link.target
    }

    /// Attach the receiver link to a session with the default configuration
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
    /// let mut receiver = ReceiverInner::attach(
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
    pub async fn recv<T>(&mut self) -> Result<Delivery<T>, RecvError>
    where
        T: DecodeIntoMessage + Send,
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

    /// Detach the link.
    ///
    /// This will send a `Detach` performative with the `closed` field set to false. If the remote
    /// peer responds with a Detach performative whose `closed` field is set to true, the link will
    /// re-attach and then close by exchanging closing Detach performatives.
    pub async fn detach(mut self) -> Result<DetachedReceiver, DetachError> {
        self.inner.detach_with_error(None).await?;
        Ok(DetachedReceiver { inner: self.inner })
    }

    /// Detach the link with an error.
    ///
    /// This will send a `Detach` performative with the `closed` field set to false. If the remote
    /// peer responds with a Detach performative whose `closed` field is set to true, the link will
    /// re-attach and then close by exchanging closing Detach performatives.
    pub async fn detach_with_error(
        mut self,
        error: impl Into<definitions::Error>,
    ) -> Result<DetachedReceiver, DetachError> {
        self.inner.detach_with_error(Some(error.into())).await?;
        Ok(DetachedReceiver { inner: self.inner })
    }

    /// Detach the link with a timeout
    ///
    /// This simply wraps [`detach`] with a `timeout`
    pub async fn detach_with_timeout(
        self,
        duration: Duration,
    ) -> Result<Result<DetachedReceiver, DetachError>, Elapsed> {
        timeout(duration, self.detach()).await
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
    /// to `Accept`
    ///
    /// # Example
    ///
    /// The code of the example below can be found in the [GitHub repo](https://github.com/minghuaw/fe2o3-amqp/blob/main/examples/receiver/src/main.rs)
    ///
    /// ```rust,ignore
    /// let delivery: Delivery<Value> = receiver.recv().await.unwrap();
    /// receiver.accept(&delivery).await.unwrap();
    /// ```
    pub async fn accept<T>(&mut self, delivery: &Delivery<T>) -> Result<(), DispositionError> {
        let state = DeliveryState::Accepted(Accepted {});
        let delivery_info = delivery.clone_info();
        self.inner.dispose(delivery_info, None, state).await
    }

    /// Accept the message by sending a disposition with the `delivery_state` field set
    /// to `Accept`
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
    pub async fn accept_all<'a, T: 'a>(
        &mut self,
        deliveries: impl IntoIterator<Item = &'a Delivery<T>>,
    ) -> Result<(), DispositionError> {
        let state = DeliveryState::Accepted(Accepted {});
        let delivery_infos = deliveries.into_iter().map(|d| d.clone_info()).collect();
        self.inner.dispose_all(delivery_infos, None, state).await
    }

    /// Reject the message by sending a disposition with the `delivery_state` field set
    /// to `Reject`
    pub async fn reject<T>(
        &mut self,
        delivery: &Delivery<T>,
        error: impl Into<Option<definitions::Error>>,
    ) -> Result<(), DispositionError> {
        let state = DeliveryState::Rejected(Rejected {
            error: error.into(),
        });
        let delivery_info = delivery.clone_info();
        self.inner.dispose(delivery_info, None, state).await
    }

    /// Reject the message by sending a disposition with the `delivery_state` field set
    /// to `Reject`
    pub async fn reject_all<'a, T: 'a>(
        &mut self,
        deliveries: impl IntoIterator<Item = &'a Delivery<T>>,
        error: impl Into<Option<definitions::Error>>,
    ) -> Result<(), DispositionError> {
        let state = DeliveryState::Rejected(Rejected {
            error: error.into(),
        });
        let delivery_infos = deliveries.into_iter().map(|d| d.clone_info()).collect();
        self.inner.dispose_all(delivery_infos, None, state).await
    }

    /// Release the message by sending a disposition with the `delivery_state` field set
    /// to `Release`
    pub async fn release<T>(&mut self, delivery: &Delivery<T>) -> Result<(), DispositionError> {
        let state = DeliveryState::Released(Released {});
        let delivery_info = delivery.clone_info();
        self.inner.dispose(delivery_info, None, state).await
    }

    /// Release the message by sending a disposition with the `delivery_state` field set
    /// to `Release`
    pub async fn release_all<'a, T: 'a>(
        &mut self,
        deliveries: impl IntoIterator<Item = &'a Delivery<T>>,
    ) -> Result<(), DispositionError> {
        let state = DeliveryState::Released(Released {});
        let delivery_infos = deliveries.into_iter().map(|d| d.clone_info()).collect();
        self.inner.dispose_all(delivery_infos, None, state).await
    }

    /// Modify the message by sending a disposition with the `delivery_state` field set
    /// to `Modify`
    pub async fn modify<T>(
        &mut self,
        delivery: &Delivery<T>,
        modified: Modified,
    ) -> Result<(), DispositionError> {
        let state = DeliveryState::Modified(modified);
        let delivery_info = delivery.clone_info();
        self.inner.dispose(delivery_info, None, state).await
    }

    /// Modify the message by sending a disposition with the `delivery_state` field set
    /// to `Modify`
    pub async fn modify_all<'a, T: 'a>(
        &mut self,
        deliveries: impl IntoIterator<Item = &'a Delivery<T>>,
        modified: Modified,
    ) -> Result<(), DispositionError> {
        let state = DeliveryState::Modified(modified);
        let delivery_infos = deliveries.into_iter().map(|d| d.clone_info()).collect();
        self.inner.dispose_all(delivery_infos, None, state).await
    }
}

#[derive(Debug)]
pub(crate) struct ReceiverInner<L: endpoint::ReceiverLink> {
    pub(crate) link: L,
    pub(crate) buffer_size: usize,
    pub(crate) credit_mode: CreditMode,
    pub(crate) processed: SequenceNo,
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

#[async_trait]
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

    fn session_control_mut(&mut self) -> &mut mpsc::Sender<SessionControl> {
        &mut self.session
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

    async fn send_detach(
        &mut self,
        closed: bool,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError> {
        self.link.send_detach(&self.outgoing, closed, error).await
    }
}

#[async_trait]
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
        T: DecodeIntoMessage + Send,
    {
        loop {
            match self.recv_inner().await? {
                Some(delivery) => return Ok(delivery),
                None => continue, // Incomplete transfer, there are more transfer frames coming
            }
        }
    }

    #[inline]
    pub(crate) async fn recv_inner<T>(&mut self) -> Result<Option<Delivery<T>>, RecvError>
    where
        T: DecodeIntoMessage + Send,
    {
        let frame = self
            .incoming
            .recv()
            .await
            .ok_or(LinkStateError::IllegalSessionState)?;

        match frame {
            LinkFrame::Detach(detach) => {
                let closed = detach.closed;
                self.link.send_detach(&self.outgoing, closed, None).await?;
                self.link
                    .on_incoming_detach(detach)
                    .await
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
            } => self.on_incoming_transfer(performative, payload).await,
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
                self.close_with_error(Some(error)).await?;
                Err(RecvError::TransactionalAcquisitionIsNotImeplemented)
            }
        }
    }

    #[inline]
    async fn on_incoming_transfer<T>(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<Delivery<T>>, RecvError>
    where
        T: DecodeIntoMessage + Send,
    {
        // Aborted messages SHOULD be discarded by the recipient (any payload
        // within the frame carrying the performative MUST be ignored). An aborted
        // message is implicitly settled
        if transfer.aborted {
            let _ = self.incomplete_transfer.take();
            return Ok(None);
        }

        let delivery = if transfer.more {
            // Partial transfer of the delivery
            match &mut self.incomplete_transfer {
                Some(incomplete) => {
                    incomplete.or_assign(transfer)?;
                    incomplete.append(payload);

                    if let Some(delivery_tag) = incomplete.performative.delivery_tag.clone() {
                        // Update unsettled map in the link
                        self.link
                            .on_incomplete_transfer(
                                delivery_tag,
                                incomplete.section_number.unwrap_or(0),
                                incomplete.section_offset,
                            )
                            .await;
                    }
                }
                None => {
                    let incomplete = IncompleteTransfer::new(transfer, payload);
                    if let Some(delivery_tag) = incomplete.performative.delivery_tag.clone() {
                        // Update unsettled map in the link
                        self.link
                            .on_incomplete_transfer(
                                delivery_tag,
                                incomplete.section_number.unwrap_or(0),
                                incomplete.section_offset,
                            )
                            .await;
                    }
                    self.incomplete_transfer = Some(Box::new(incomplete));
                }
            }

            // Partial delivery doesn't yield a complete message
            return Ok(None);
        } else {
            // Final transfer of the delivery
            match self.incomplete_transfer.take() {
                Some(mut incomplete) => {
                    incomplete.or_assign(transfer)?;
                    incomplete.append(payload); // This also computes the section number and offset incrementally

                    self.link
                        .on_complete_transfer(
                            incomplete.performative,
                            incomplete.buffer,
                            incomplete.section_number.unwrap_or(0),
                            incomplete.section_offset,
                        )
                        .await?
                }
                None => {
                    // let message: Message = from_reader(payload.reader())?;
                    // TODO: Is there any way to optimize this?
                    // let (section_number, section_offset) = section_number_and_offset(payload.as_ref());
                    let (section_number, section_offset) =
                        count_number_of_sections_and_offset(&payload);
                    self.link
                        .on_complete_transfer(transfer, payload, section_number, section_offset)
                        .await?
                }
            }
        };

        // Auto accept the message and leave settled to be determined based on rcv_settle_mode
        if self.auto_accept {
            let delivery_info = delivery.clone_info();
            self.dispose(delivery_info, None, Accepted {}.into())
                .await?;
        }

        Ok(Some(delivery))
    }

    /// Set the link credit. This will stop draining if the link is in a draining cycle
    #[inline]
    pub async fn set_credit(&mut self, credit: SequenceNo) -> Result<(), IllegalLinkStateError> {
        self.processed = 0;
        if let CreditMode::Auto(_) = self.credit_mode {
            self.credit_mode = CreditMode::Auto(credit)
        }

        self.link
            .send_flow(&self.outgoing, Some(credit), Some(false), false)
            .await
    }

    // TODO: batch disposition
    #[inline]
    pub(crate) async fn dispose(
        &mut self,
        delivery_info: DeliveryInfo,
        settled: Option<bool>,
        state: DeliveryState,
    ) -> Result<(), DispositionError> {
        self.link
            .dispose(&self.outgoing, delivery_info, settled, state, false)
            .await?;

        self.processed += 1;
        self.update_credit_if_auto().await?;
        Ok(())
    }

    #[inline]
    pub(crate) async fn dispose_all(
        &mut self,
        delivery_infos: Vec<DeliveryInfo>,
        settled: Option<bool>,
        state: DeliveryState,
    ) -> Result<(), DispositionError> {
        let total = delivery_infos.len();
        self.link
            .dispose_all(&self.outgoing, delivery_infos, settled, state, false)
            .await?;

        self.processed += total as u32;
        self.update_credit_if_auto().await?;
        Ok(())
    }

    #[inline]
    async fn update_credit_if_auto(&mut self) -> Result<(), DispositionError> {
        if let CreditMode::Auto(max_credit) = self.credit_mode {
            if self.processed >= max_credit / 2 {
                // self.processed will be set to zero when setting link credit
                self.set_credit(max_credit).await?;
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
        self.processed = 0;

        // Return if already draining
        if self.link.flow_state().drain().await {
            return Ok(());
        }

        // Send a flow with Drain set to true
        self.link
            .send_flow(&self.outgoing, None, Some(true), false)
            .await
    }
}

/// A detached receiver
///
/// # Link re-attachment
///
/// TODO
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
    async fn resume_inner(
        &mut self,
        mut remote_attach: Option<Attach>,
    ) -> Result<ReceiverAttachExchange, ReceiverResumeErrorKind> {
        self.inner.reallocate_output_handle().await?;

        let exchange = match remote_attach.take() {
            Some(remote_attach) => {
                self.inner
                    .link
                    .send_attach(&self.inner.outgoing, &self.inner.session, false)
                    .await?;
                self.inner.link.on_incoming_attach(remote_attach).await?
            }
            None => self.inner.exchange_attach(false).await?,
        };
        tracing::debug!(?exchange);

        let credit = self.inner.link.flow_state.link_credit().await;
        self.inner.set_credit(credit).await?;

        Ok(exchange)
    }

    /// Resume the receiver link
    ///
    /// Please note that the link may need to be detached and then resume multiple
    /// times if there are unsettled deliveries.
    #[instrument(skip(self))]
    pub async fn resume(mut self) -> Result<ResumingReceiver, ReceiverResumeError> {
        let exchange = try_as_recver!(self, self.resume_inner(None).await);
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

    /// Resume the receiver link with a timeout.
    ///
    /// Upon failure, the detached receiver can be accessed via `error.detached_recver`
    ///
    /// Please note that the link may need to be detached and then resume multiple
    /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
    #[instrument(skip(self))]
    pub async fn resume_with_timeout(
        mut self,
        duration: Duration,
    ) -> Result<ResumingReceiver, ReceiverResumeError> {
        let fut = self.resume_inner(None);

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

    /// Resume the receiver on a specific session
    ///
    /// Please note that the link may need to be detached and then resume multiple
    /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
    pub async fn resume_on_session<R>(
        mut self,
        session: &SessionHandle<R>,
    ) -> Result<ResumingReceiver, ReceiverResumeError> {
        *self.inner.session_control_mut() = session.control.clone();
        self.resume().await
    }

    /// Resume the receiver on a specific session with timeout
    ///
    /// Please note that the link may need to be detached and then resume multiple
    /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
    pub async fn resume_on_session_with_timeout<R>(
        mut self,
        session: &SessionHandle<R>,
        duration: Duration,
    ) -> Result<ResumingReceiver, ReceiverResumeError> {
        *self.inner.session_control_mut() = session.control.clone();
        self.resume_with_timeout(duration).await
    }

    /// Resume the receiver link on the original session with an Attach sent by the remote peer
    ///
    /// Please note that the link may need to be detached and then resume multiple
    /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
    pub async fn resume_incoming_attach(
        mut self,
        remote_attach: Attach,
    ) -> Result<ResumingReceiver, ReceiverResumeError> {
        let exchange = try_as_recver!(self, self.resume_inner(Some(remote_attach)).await);
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

    /// Resume the receiver link on the original session with an Attach sent by the remote peer
    ///
    /// Please note that the link may need to be detached and then resume multiple
    /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
    pub async fn resume_incoming_attach_with_timeout(
        mut self,
        remote_attach: Attach,
        duration: Duration,
    ) -> Result<ResumingReceiver, ReceiverResumeError> {
        let fut = self.resume_inner(Some(remote_attach));

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

    /// Resume the receiver on a specific session
    ///
    /// Please note that the link may need to be detached and then resume multiple
    /// times if there are unsettled deliveries. For more details please see [`resume`](./#method.resume)
    pub async fn resume_incoming_attach_on_session<R>(
        mut self,
        remote_attach: Attach,
        session: &SessionHandle<R>,
    ) -> Result<ResumingReceiver, ReceiverResumeError> {
        *self.inner.session_control_mut() = session.control.clone();
        self.resume_incoming_attach(remote_attach).await
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
        *self.inner.session_control_mut() = session.control.clone();
        self.resume_incoming_attach_with_timeout(remote_attach, duration)
            .await
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp_types::performatives::Transfer;

    use super::IncompleteTransfer;

    #[test]
    fn size_of_incomplete_transfer() {
        let size = std::mem::size_of::<Transfer>();
        println!("Transfer {:?}", size);

        let size = std::mem::size_of::<Option<IncompleteTransfer>>();
        println!("Option<IncompleteTransfer> {:?}", size);
    }
}
