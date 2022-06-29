//! Implementation of AMQP1.0 receiver

use std::time::Duration;

use async_trait::async_trait;
use bytes::BytesMut;
use fe2o3_amqp_types::{
    definitions::{self, AmqpError, DeliveryNumber, DeliveryTag, SequenceNo},
    messaging::{Accepted, Address, DeliveryState, Modified, Rejected, Released, Target},
    performatives::{Detach, Transfer},
};
use tokio::{
    sync::mpsc,
    time::{error::Elapsed, timeout},
};

use crate::{
    control::SessionControl,
    endpoint::{self, LinkAttach, LinkDetach, LinkExt},
    session::SessionHandle,
    Payload,
};

use super::{
    builder::{self, WithTarget, WithoutName, WithoutSource},
    delivery::Delivery,
    error::DetachError,
    receiver_link::section_number_and_offset,
    role,
    shared_inner::{LinkEndpointInner, LinkEndpointInnerDetach},
    ArcReceiverUnsettledMap, DispositionError, IllegalLinkStateError, LinkFrame, LinkRelay,
    LinkStateError, ReceiverAttachError, ReceiverFlowState, ReceiverLink, ReceiverTransferError,
    RecvError, DEFAULT_CREDIT,
};

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
    pub buffer: BytesMut,
    pub section_number: u32,
    pub section_offset: u64,
}

impl IncompleteTransfer {
    pub fn new(transfer: Transfer, partial_payload: Payload) -> Self {
        let (number, offset) = section_number_and_offset(partial_payload.as_ref());
        let mut buffer = BytesMut::new();
        // TODO: anyway to make this not copying the bytes?
        buffer.extend(partial_payload);
        Self {
            performative: transfer,
            buffer,
            section_number: number,
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
        let (number, offset) = section_number_and_offset(other.as_ref());
        self.section_number += number;
        if number == 0 {
            // No new sections has been transmitted
            self.section_offset += offset;
        } else {
            // New section(s) has been transmitted
            self.section_offset = offset;
        }

        self.buffer.extend(other);
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
///
/// # Customize configuration with [`builder::Builder`]
///
/// ```rust, ignore
/// let mut receiver = ReceiverInner::builder()
///     .name("rust-receiver-link-1")
///     .source("q1")
///     .attach(&mut session)
///     .await
///     .unwrap();
/// ```
#[derive(Debug)]
pub struct Receiver {
    pub(crate) inner: ReceiverInner<ReceiverLink<Target>>,
}

impl Receiver {
    /// Creates a builder for the [`ReceiverInner`]
    pub fn builder() -> builder::Builder<role::Receiver, Target, WithoutName, WithoutSource, WithTarget> {
        builder::Builder::<role::Receiver, Target, _, _, _>::new()
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
    ///
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
        T: for<'de> serde::Deserialize<'de> + Send,
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
        Ok(DetachedReceiver { _inner: self.inner })
    }

    /// Detach the link with an error.
    ///
    /// This will send a `Detach` performative with the `closed` field set to false. If the remote
    /// peer responds with a Detach performative whose `closed` field is set to true, the link will
    /// re-attach and then close by exchanging closing Detach performatives.
    pub async fn detach_with_error(
        mut self,
        error: impl Into<Option<definitions::Error>>,
    ) -> Result<DetachedReceiver, DetachError> {
        self.inner.detach_with_error(error.into()).await?;
        Ok(DetachedReceiver { _inner: self.inner })
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
        error: impl Into<Option<definitions::Error>>,
    ) -> Result<(), DetachError> {
        self.inner.close_with_error(error.into()).await
    }

    /// Accept the message by sending a disposition with the `delivery_state` field set
    /// to `Accept`
    pub async fn accept<T>(&mut self, delivery: &Delivery<T>) -> Result<(), DispositionError> {
        let state = DeliveryState::Accepted(Accepted {});
        self.inner
            .dispose(
                delivery.delivery_id,
                delivery.delivery_tag.clone(),
                None,
                state,
            )
            .await
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
        self.inner
            .dispose(
                delivery.delivery_id,
                delivery.delivery_tag.clone(),
                None,
                state,
            )
            .await
    }

    /// Release the message by sending a disposition with the `delivery_state` field set
    /// to `Release`
    pub async fn release<T>(&mut self, delivery: &Delivery<T>) -> Result<(), DispositionError> {
        let state = DeliveryState::Released(Released {});
        self.inner
            .dispose(
                delivery.delivery_id,
                delivery.delivery_tag.clone(),
                None,
                state,
            )
            .await
    }

    /// Modify the message by sending a disposition with the `delivery_state` field set
    /// to `Modify`
    pub async fn modify<T>(
        &mut self,
        delivery: &Delivery<T>,
        modified: Modified,
    ) -> Result<(), DispositionError> {
        let state = DeliveryState::Modified(modified);
        self.inner
            .dispose(
                delivery.delivery_id,
                delivery.delivery_tag.clone(),
                None,
                state,
            )
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
    _inner: ReceiverInner<ReceiverLink<Target>>,
}

#[derive(Debug)]
pub(crate) struct ReceiverInner<L: endpoint::ReceiverLink> {
    pub(crate) link: L,
    pub(crate) buffer_size: usize,
    pub(crate) credit_mode: CreditMode,
    pub(crate) processed: SequenceNo,

    // Control sender to the session
    pub(crate) session: mpsc::Sender<SessionControl>,

    // Outgoing mpsc channel to send the Link Frames
    pub(crate) outgoing: mpsc::Sender<LinkFrame>,
    pub(crate) incoming: mpsc::Receiver<LinkFrame>,

    pub(crate) incomplete_transfer: Option<IncompleteTransfer>,
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

impl<L> ReceiverInner<L>
where
    L: endpoint::ReceiverLink<
            FlowError = IllegalLinkStateError,
            TransferError = ReceiverTransferError,
            DispositionError = IllegalLinkStateError,
            AttachError = ReceiverAttachError,
            DetachError = DetachError,
        > + LinkExt<FlowState = ReceiverFlowState, Unsettled = ArcReceiverUnsettledMap>
        + Send
        + Sync,
{
    pub(crate) async fn recv<T>(&mut self) -> Result<Delivery<T>, RecvError>
    where
        T: for<'de> serde::Deserialize<'de> + Send,
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
        T: for<'de> serde::Deserialize<'de> + Send,
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
                // self.link.send_detach(&self.outgoing, closed, None).await
                //     .map_err(Into::into)
                //     .and_then(|_| match closed {
                //         true => Err(LinkStateError::RemoteClosed.into()),
                //         false => Err(LinkStateError::RemoteDetached.into()),
                //     })
                // match (detach.error, closed) {
                //     (Some(err), false) => Err(LinkStateError::RemoteDetachedWithError(err).into()),
                //     (Some(err), true) => Err(LinkStateError::RemoteClosedWithError(err).into()),
                //     (None, false) => Err(LinkStateError::RemoteDetached.into()),
                //     (None, true) => Err(LinkStateError::RemoteClosed.into()),
                // }
            }
            LinkFrame::Transfer {
                input_handle: _,
                performative,
                payload,
            } => self.on_incoming_transfer(performative, payload).await,
            LinkFrame::Acquisition(_) => {
                let error = definitions::Error::new(
                    AmqpError::NotImplemented,
                    "Transactional acquisition is not implemented".to_string(),
                    None,
                );
                self.close_with_error(Some(error)).await?;
                Err(RecvError::TransactionalAcquisitionIsNotImeplemented)
            }
            LinkFrame::Attach(_) => Err(LinkStateError::IllegalState.into()),
            LinkFrame::Flow(_) | LinkFrame::Disposition(_) => {
                // Flow and Disposition are handled by LinkRelay which runs
                // in the session loop
                unreachable!()
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
        T: for<'de> serde::Deserialize<'de> + Send,
    {
        // Aborted messages SHOULD be discarded by the recipient (any payload
        // within the frame carrying the performative MUST be ignored). An aborted
        // message is implicitly settled
        if transfer.aborted {
            let _ = self.incomplete_transfer.take();
            return Ok(None);
        }

        let (delivery, disposition) = if transfer.more {
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
                                incomplete.section_number,
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
                                incomplete.section_number,
                                incomplete.section_offset,
                            )
                            .await;
                    }
                    self.incomplete_transfer = Some(incomplete);
                }
            }

            // Partial delivery doesn't yield a complete message
            return Ok(None);
        } else {
            // Final transfer of the delivery
            match self.incomplete_transfer.take() {
                Some(mut incomplete) => {
                    incomplete.or_assign(transfer)?;
                    let IncompleteTransfer {
                        performative,
                        mut buffer,
                        ..
                    } = incomplete;
                    buffer.extend(payload);
                    self.link
                        .on_complete_transfer(performative, buffer.freeze())
                        .await?
                }
                None => {
                    // let message: Message = from_reader(payload.reader())?;
                    // TODO: Is there any way to optimize this?
                    // let (section_number, section_offset) = section_number_and_offset(payload.as_ref());
                    self.link.on_complete_transfer(transfer, payload).await?
                }
            }
        };

        // In `ReceiverSettleMode::First`, if the message is not pre-settled
        // the receiver will spontaneously settle the message with an
        // Accept by returning a `Some(Disposition)`
        if let Some((delivery_id, delivery_tag, delivery_state)) = disposition {
            // let frame = LinkFrame::Disposition(disposition);
            // self.outgoing.send(frame).await?;
            self.dispose(delivery_id, delivery_tag, None, delivery_state)
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
            .send_flow(&mut self.outgoing, Some(credit), Some(false), false)
            .await
    }

    // TODO: batch disposition
    #[inline]
    pub(crate) async fn dispose(
        &mut self,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        settled: Option<bool>,
        state: DeliveryState,
    ) -> Result<(), DispositionError> {
        let _ = self
            .link
            .dispose(
                &mut self.outgoing,
                delivery_id,
                delivery_tag,
                settled,
                state,
                false,
            )
            .await?;

        self.processed += 1;
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
            .send_flow(&mut self.outgoing, None, Some(true), false)
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
