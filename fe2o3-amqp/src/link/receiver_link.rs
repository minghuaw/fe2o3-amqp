use fe2o3_amqp_types::{
    definitions::Handle,
    messaging::{message::DecodeIntoMessage, FromBody},
};
use serde_amqp::format_code::EncodingCodes;

use crate::util::{is_consecutive, AsByteIterator, IntoReader};

use super::{delivery::DeliveryInfo, *};

pub(crate) const DESCRIBED_TYPE: u8 = EncodingCodes::DescribedType as u8;
pub(crate) const SMALL_ULONG_TYPE: u8 = EncodingCodes::SmallULong as u8;
pub(crate) const ULONG_TYPE: u8 = EncodingCodes::ULong as u8;
pub(crate) const HEADER_CODE: u8 = 0x70;
pub(crate) const DELIV_ANNOT_CODE: u8 = 0x71;
pub(crate) const MSG_ANNOT_CODE: u8 = 0x72;
pub(crate) const PROP_CODE: u8 = 0x73;
pub(crate) const APP_PROP_CODE: u8 = 0x74;
pub(crate) const DATA_CODE: u8 = 0x75;
pub(crate) const AMQP_SEQ_CODE: u8 = 0x76;
pub(crate) const AMQP_VAL_CODE: u8 = 0x77;
pub(crate) const FOOTER_CODE: u8 = 0x78;

#[async_trait]
impl<Tar> endpoint::ReceiverLink for ReceiverLink<Tar>
where
    Tar: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync,
{
    type FlowError = FlowError;
    type TransferError = ReceiverTransferError;
    type DispositionError = DispositionError;

    /// Set and send flow state
    ///
    /// This is cancel safe because it only `.await` on sending over a `tokio::mpsc::Sender`
    async fn send_flow(
        &self,
        writer: &mpsc::Sender<LinkFrame>,
        link_credit: Option<u32>,
        drain: Option<bool>,
        echo: bool,
    ) -> Result<(), Self::FlowError> {
        let handle = self
            .output_handle
            .clone()
            .ok_or(Self::FlowError::IllegalState)?
            .into();

        let flow = self.get_link_flow(handle, link_credit, drain, echo);
        writer
            .send(LinkFrame::Flow(flow))
            .await // cancel safe
            .map_err(|_| Self::FlowError::IllegalSessionState)
    }

    fn on_transfer_state(
        &mut self,
        delivery_tag: &Option<DeliveryTag>,
        settled: Option<bool>,
        state: DeliveryState,
    ) -> Result<(), Self::TransferError> {
        let delivery_tag = delivery_tag
            .as_ref()
            .ok_or(Self::TransferError::DeliveryTagIsNone)?;
        let mut guard = self.unsettled.write();
        let map = guard.get_or_insert(OrderedMap::new());

        if matches!(settled, Some(true)) {
            // FIXME: Simply remove from the unsettled map?
            let _ = map.remove(delivery_tag);
        } else {
            // If a terminal state is already achieved, cannot send any further
            let value = map.get_mut(delivery_tag);
            match value {
                Some(val) => {
                    match val.as_ref().map(|v| v.is_terminal()) {
                        Some(true) => {
                            // Note that if the transfer performative (or an earlier disposition performative referring to the
                            //     delivery) indicates that the delivery has attained a terminal state, then no future transfer or
                            //     disposition sent by the sender can alter that terminal state.
                            return Ok(());
                        }
                        _ => *val = Some(state),
                    }
                }
                None => {
                    map.insert(delivery_tag.clone(), Some(state));
                }
            }
        }
        Ok(())
    }

    fn on_incomplete_transfer(
        &mut self,
        delivery_tag: DeliveryTag,
        section_number: u32,
        section_offset: u64,
    ) {
        // link-credit is defined as
        // "The current maximum number of messages that can be handled
        // at the receiver endpoint of the link"
        // So there is no need to decrement the link-credit on incomplete delivery

        let state = DeliveryState::Received(Received {
            section_number,
            section_offset,
        });

        {
            let mut guard = self.unsettled.write();
            // The same key may be writter multiple times
            let _ = guard
                .get_or_insert(OrderedMap::new())
                .insert(delivery_tag, Some(state));
        }
    }

    fn on_complete_transfer<'a, T, P>(
        &'a mut self,
        transfer: Transfer,
        payload: P,
        section_number: u32,
        section_offset: u64,
    ) -> Result<Delivery<T>, Self::TransferError>
    where
        for<'de> T: FromBody<'de> + Send,
        for<'b> P: IntoReader + AsByteIterator<'b> + Send + 'a,
    {
        match self.local_state {
            LinkState::Attached | LinkState::IncompleteAttachExchanged => {}
            _ => return Err(ReceiverTransferError::IllegalState),
        }

        // ReceiverFlowState will not wait until link credit is available.
        // Will return with an error if there is not enough link credit.
        self.flow_state.consume(1)?;

        // This only takes care of whether the message is considered
        // sett
        let settled_by_sender = transfer.settled.unwrap_or(false);
        let delivery_id = transfer
            .delivery_id
            .ok_or(Self::TransferError::DeliveryIdIsNone)?;
        let delivery_tag = transfer
            .delivery_tag
            .ok_or(Self::TransferError::DeliveryTagIsNone)?;

        let (message, mode) = if settled_by_sender {
            // If the message is pre-settled, there is no need to
            // add to the unsettled map and no need to reply to the Sender
            let message = T::decode_into_message(payload.into_reader())
                .map_err(|_| Self::TransferError::MessageDecodeError)?;
            (message, None)
        } else {
            // If the message is being sent settled by the sender, the value of this
            // field is ignored.
            let mode = match transfer.rcv_settle_mode {
                Some(mode) => {
                    // If the negotiated link value is first, then it is illegal to set this
                    // field to second.
                    if matches!(&self.rcv_settle_mode, ReceiverSettleMode::First)
                        && matches!(mode, ReceiverSettleMode::Second)
                    {
                        return Err(Self::TransferError::IllegalRcvSettleModeInTransfer);
                    }
                    Some(mode)
                }
                None => None,
            };

            let message = T::decode_into_message(payload.into_reader())
                .map_err(|_| Self::TransferError::MessageDecodeError)?;

            let state = DeliveryState::Received(Received {
                section_number, // What is section number?
                section_offset,
            });

            // Upon receiving the transfer, the receiving link endpoint (receiver)
            // will create an entry in its own unsettled map and make the transferred
            // message data available to the application to process.
            //
            // Add to unsettled map
            // Insert into local unsettled map with Received state
            // Mode Second doesn't automatically send back a disposition
            // (ie. thus doesn't call `link.dispose()`) and thus need to manually
            // set the delivery state
            {
                let mut lock = self.unsettled.write();
                // There may be records of incomplete delivery
                let _ = lock
                    .get_or_insert(OrderedMap::new())
                    .insert(delivery_tag.clone(), Some(state));
            }
            (message, mode)
        };

        let link_output_handle = self
            .output_handle
            .clone()
            .ok_or(ReceiverTransferError::IllegalState)?
            .into();

        let delivery = Delivery {
            link_output_handle,
            delivery_id,
            delivery_tag,
            rcv_settle_mode: mode,
            message,
        };

        Ok(delivery)
    }

    /// This is cancel safe because it only `.await` on sending over `tokio::mpsc::Sender`
    async fn dispose(
        &self,
        writer: &mpsc::Sender<LinkFrame>,
        delivery_info: DeliveryInfo,
        settled: Option<bool>,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError> {
        let settled = settled.unwrap_or({
            match delivery_info
                .rcv_settle_mode
                .as_ref()
                .unwrap_or(&self.rcv_settle_mode)
            {
                ReceiverSettleMode::First => {
                    // If first, this indicates that the receiver MUST settle
                    // the delivery once it has arrived without waiting
                    // for the sender to settle first.

                    // The delivery is not inserted into unsettled map if in First mode
                    true
                }
                ReceiverSettleMode::Second => {
                    // If second, this indicates that the receiver MUST NOT settle until sending
                    // its disposition to the sender and receiving a settled disposition from
                    // the sender.
                    false
                }
            }
        });

        let unsettled_state = if settled {
            let mut lock = self.unsettled.write();
            lock.as_mut()
                .and_then(|map| map.remove(&delivery_info.delivery_tag))
        } else {
            let mut lock = self.unsettled.write();
            // If the key is present in the map, the old value will be returned, which
            // we don't really need
            lock.get_or_insert(OrderedMap::new())
                .insert(delivery_info.delivery_tag.clone(), Some(state.clone()))
        };

        // Only dispose if message is found in unsettled map
        if unsettled_state.is_some() {
            let disposition = Disposition {
                role: Role::Receiver,
                first: delivery_info.delivery_id,
                last: None,
                settled,
                state: Some(state),
                batchable,
            };
            let frame = LinkFrame::Disposition(disposition);
            writer
                .send(frame)
                .await // cancel safe
                .map_err(|_| Self::DispositionError::IllegalSessionState)?;
        }

        Ok(())
    }

    /// This is cancel safe because all internal `.await` points are cancel safe
    async fn dispose_all(
        &self,
        writer: &mpsc::Sender<LinkFrame>,
        mut delivery_infos: Vec<DeliveryInfo>,
        settled: Option<bool>,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError> {
        // sorting before filtering may be more cache/branch-prediction friendly?
        delivery_infos.sort_by(|left, right| left.delivery_id.cmp(&right.delivery_id));
        {
            let reader = self.unsettled.read();
            delivery_infos.retain(|info| {
                reader
                    .as_ref()
                    .map(|m| m.contains_key(&info.delivery_tag))
                    .unwrap_or(false)
            });
        }
        let chunk_inds = consecutive_chunk_indices(&delivery_infos);

        let mut prev_ind = 0;
        for ind in chunk_inds {
            let slice = &delivery_infos[prev_ind..ind];
            self.dispose_consecutive(writer, slice, settled, state.clone(), batchable)
                .await?; // cancel safe
            prev_ind = ind;
        }
        let final_slice = &delivery_infos[prev_ind..];
        self.dispose_consecutive(writer, final_slice, settled, state, batchable)
            .await // cancel safe
    }
}

fn consecutive_chunk_indices(delivery_infos: &[DeliveryInfo]) -> Vec<usize> {
    delivery_infos
        .windows(2)
        .enumerate()
        .filter_map(|(i, infos)| {
            if is_consecutive(&infos[0].delivery_id, &infos[1].delivery_id)
                && infos[0].rcv_settle_mode == infos[1].rcv_settle_mode
            {
                None
            } else {
                // window size is 2, so the iter is 1 less than the total len
                Some(i + 1)
            }
        })
        .collect()
}

/// Count number of sections in encoded message
pub(crate) fn count_number_of_sections_and_offset<'a, B>(bytes: &'a B) -> (u32, u64)
where
    B: AsByteIterator<'a>,
{
    let b0 = bytes.as_byte_iterator();
    let len = b0.len();
    let b1 = bytes.as_byte_iterator().skip(1);
    let b2 = bytes.as_byte_iterator().skip(2);
    let iter = b0.zip(b1.zip(b2));

    let mut last_pos = 0;
    let mut section_numbers = 0;

    for (i, (&b0, (&b1, &b2))) in iter.enumerate() {
        if is_section_header(b0, b1, b2) {
            section_numbers += 1;
            last_pos = i;
        }
    }

    let offset = len - last_pos;
    (section_numbers, offset as u64)
}

pub(crate) fn is_section_header(b0: u8, b1: u8, b2: u8) -> bool {
    matches!(
        (b0, b1, b2),
        (DESCRIBED_TYPE, SMALL_ULONG_TYPE, HEADER_CODE)
        | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, DELIV_ANNOT_CODE)
        | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, MSG_ANNOT_CODE)
        | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, PROP_CODE)
        | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, APP_PROP_CODE)
        | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, DATA_CODE)
        | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, AMQP_SEQ_CODE)
        | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, AMQP_VAL_CODE)
        | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, FOOTER_CODE)
        // Some implementation may use ULong for all u64 numbers
        | (DESCRIBED_TYPE, ULONG_TYPE, HEADER_CODE)
        | (DESCRIBED_TYPE, ULONG_TYPE, DELIV_ANNOT_CODE)
        | (DESCRIBED_TYPE, ULONG_TYPE, MSG_ANNOT_CODE)
        | (DESCRIBED_TYPE, ULONG_TYPE, PROP_CODE)
        | (DESCRIBED_TYPE, ULONG_TYPE, APP_PROP_CODE)
        | (DESCRIBED_TYPE, ULONG_TYPE, DATA_CODE)
        | (DESCRIBED_TYPE, ULONG_TYPE, AMQP_SEQ_CODE)
        | (DESCRIBED_TYPE, ULONG_TYPE, AMQP_VAL_CODE)
        | (DESCRIBED_TYPE, ULONG_TYPE, FOOTER_CODE)
    )
}

impl ReceiverLink<Target> {
    /// Set and send flow state
    #[cfg(feature = "transaction")]
    pub(crate) fn blocking_send_flow(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        link_credit: Option<u32>,
        drain: Option<bool>,
        echo: bool,
    ) -> Result<(), FlowError> {
        let handle = self
            .output_handle
            .clone()
            .ok_or(FlowError::IllegalState)?
            .into();

        let flow = self.get_link_flow(handle, link_credit, drain, echo);
        writer
            .blocking_send(LinkFrame::Flow(flow))
            .map_err(|_| FlowError::IllegalSessionState)
    }
}

impl<T> ReceiverLink<T> {
    fn handle_unsettled_in_attach(
        &mut self,
        remote_unsettled: Option<OrderedMap<DeliveryTag, Option<DeliveryState>>>,
    ) -> ReceiverAttachExchange {
        let remote_is_empty = match remote_unsettled {
            Some(map) => map.is_empty(),
            None => true,
        };

        // ActiveMQ-Artemis seems like ignores non-empty unsettled from receiver-link
        if remote_is_empty {
            return ReceiverAttachExchange::Complete;
        }

        match self.local_state {
            LinkState::IncompleteAttachReceived
            | LinkState::IncompleteAttachSent
            | LinkState::IncompleteAttachExchanged => ReceiverAttachExchange::IncompleteUnsettled,
            _ => ReceiverAttachExchange::Resume,
        }
    }

    /// This is cancel safe because it only `.await` on sending over a `tokio::mpsc::Sender`
    async fn dispose_consecutive(
        &self,
        writer: &mpsc::Sender<LinkFrame>,
        consecutive_infos: &[DeliveryInfo],
        settled: Option<bool>,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), DispositionError> {
        // This shouldn't happen but just being cautious
        if consecutive_infos.is_empty() {
            return Ok(());
        }

        let settled = settled.unwrap_or({
            match consecutive_infos[0]
                .rcv_settle_mode
                .as_ref()
                .unwrap_or(&self.rcv_settle_mode)
            {
                ReceiverSettleMode::First => true,
                ReceiverSettleMode::Second => false,
            }
        });

        // TODO: Individually checking whether a delivery is already dropped is probably too heavy?
        if settled {
            let mut lock = self.unsettled.write();
            for info in consecutive_infos {
                lock.as_mut().and_then(|map| map.remove(&info.delivery_tag));
            }
        } else {
            let mut lock = self.unsettled.write();
            for info in consecutive_infos {
                lock.get_or_insert(OrderedMap::new())
                    .insert(info.delivery_tag.clone(), Some(state.clone()));
            }
        }

        let disposition = Disposition {
            role: Role::Receiver,
            first: consecutive_infos[0].delivery_id,
            last: consecutive_infos.last().map(|el| el.delivery_id),
            settled,
            state: Some(state),
            batchable,
        };
        let frame = LinkFrame::Disposition(disposition);
        writer
            .send(frame)
            .await // cancel safe
            .map_err(|_| DispositionError::IllegalSessionState)
    }

    fn get_link_flow(
        &self,
        handle: Handle,
        link_credit: Option<u32>,
        drain: Option<bool>,
        echo: bool,
    ) -> LinkFlow {
        match (link_credit, drain) {
            (Some(link_credit), Some(drain)) => {
                let mut guard = self.flow_state.lock.write();
                guard.link_credit = link_credit;
                guard.drain = drain;
                LinkFlow {
                    handle,
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(guard.delivery_count),
                    link_credit: Some(link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain,
                    echo,
                    properties: guard.properties.clone(),
                }
            }
            (Some(link_credit), None) => {
                let mut guard = self.flow_state.lock.write();
                guard.link_credit = link_credit;
                LinkFlow {
                    handle,
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(guard.delivery_count),
                    link_credit: Some(link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain: guard.drain,
                    echo,
                    properties: guard.properties.clone(),
                }
            }
            (None, Some(drain)) => {
                let mut guard = self.flow_state.lock.write();
                guard.drain = drain;
                LinkFlow {
                    handle,
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(guard.delivery_count),
                    link_credit: Some(guard.link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain,
                    echo,
                    properties: guard.properties.clone(),
                }
            }
            (None, None) => {
                let guard = self.flow_state.lock.read();
                LinkFlow {
                    handle,
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(guard.delivery_count),
                    link_credit: Some(guard.link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain: guard.drain,
                    echo,
                    properties: guard.properties.clone(),
                }
            }
        }
    }
}

#[async_trait]
impl<T> endpoint::LinkAttach for ReceiverLink<T>
where
    T: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync,
{
    type AttachExchange = ReceiverAttachExchange;
    type AttachError = ReceiverAttachError;

    fn on_incoming_attach(
        &mut self,
        remote_attach: Attach,
    ) -> Result<Self::AttachExchange, Self::AttachError> {
        use self::source::VerifySource;

        match (&self.local_state, remote_attach.incomplete_unsettled) {
            (LinkState::AttachSent, false) => {
                self.local_state = LinkState::Attached;
            }
            (LinkState::IncompleteAttachSent, false) => {
                self.local_state = LinkState::IncompleteAttachExchanged;
            }
            (LinkState::Unattached, false) | (LinkState::Detached, false) => {
                self.local_state = LinkState::AttachReceived; // re-attaching
            }
            (LinkState::AttachSent, true) | (LinkState::IncompleteAttachSent, true) => {
                self.local_state = LinkState::IncompleteAttachExchanged;
            }
            (LinkState::Unattached, true) | (LinkState::Detached, true) => {
                self.local_state = LinkState::IncompleteAttachReceived; // re-attaching
            }
            _ => return Err(ReceiverAttachError::IllegalState),
        };

        self.input_handle = Some(InputHandle::from(remote_attach.handle));

        // In this case, the sender is considered to hold the authoritative version of the
        // version of the source properties
        let remote_source = remote_attach
            .source
            // Only need to check the source
            //
            // If there is no pre-existing terminus, and the peer does not wish to create a new one,
            // this is indicated by setting the local terminus (source or target as appropriate) to null.
            .ok_or(ReceiverAttachError::IncomingSourceIsNone)?;
        if let Some(local_source) = &self.source {
            local_source.verify_as_receiver(&remote_source)?;
        }
        self.source = Some(*remote_source);

        // When set at the sender this indicates the actual settlement mode in use
        //
        // The receiver doesn't really care what snd_settle_mode is in use. It uses
        // the `settled` field of the Transfer and rcv_settle_mode to decide whether a
        // delivery is settled
        self.snd_settle_mode = remote_attach.snd_settle_mode;

        // When set at the receiver this indicates the actual settlement mode in use
        if self.rcv_settle_mode != remote_attach.rcv_settle_mode {
            return Err(ReceiverAttachError::RcvSettleModeNotSupported);
        }

        // The delivery-count is initialized by the sender when a link endpoint is
        // created, and is incremented whenever a message is sent
        let initial_delivery_count = remote_attach
            .initial_delivery_count
            .ok_or(ReceiverAttachError::InitialDeliveryCountIsNone)?;

        let target = remote_attach
            .target
            .map(|t| T::try_from(*t))
            .transpose()
            .map_err(|_| ReceiverAttachError::CoordinatorIsNotImplemented)?;

        // **the receiver is considered to hold the authoritative version of the target properties**,
        // Is this verification necessary?
        //
        // Only need to check the source
        //
        // If there is no pre-existing terminus, and the peer does not wish to create a new one,
        // this is indicated by setting the local terminus (source or target as appropriate) to null.
        if let (Some(local_target), Some(remote_target)) = (&self.target, &target) {
            local_target.verify_as_receiver(remote_target)?
        }

        self.max_message_size =
            get_max_message_size(self.max_message_size, remote_attach.max_message_size);

        self.flow_state
            .as_ref()
            .initial_delivery_count_mut(|_| initial_delivery_count);
        self.flow_state
            .as_ref()
            .delivery_count_mut(|_| initial_delivery_count);

        // Ok(Self::AttachExchange::Complete)
        Ok(self.handle_unsettled_in_attach(remote_attach.unsettled))
    }

    /// # Cancel safety
    ///
    /// This is cancel safe if oneshot channel is cancel safe
    async fn send_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        session: &mpsc::Sender<SessionControl>,
        is_reattaching: bool,
    ) -> Result<(), Self::AttachError> {
        self.send_attach_inner(writer, session, is_reattaching)
            .await?; // FIXME: cancel safe? if oneshot channel is cancel safe
        Ok(())
    }
}

impl<T> endpoint::Link for ReceiverLink<T>
where
    T: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync,
{
    fn role() -> Role {
        Role::Receiver
    }
}

#[async_trait]
impl<T> endpoint::LinkExt for ReceiverLink<T>
where
    T: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync,
{
    type FlowState = ReceiverFlowState;
    type Unsettled = ArcReceiverUnsettledMap;
    type Target = T;

    fn local_state(&self) -> &LinkState {
        &self.local_state
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_handle(&self) -> &Option<OutputHandle> {
        &self.output_handle
    }

    fn output_handle_mut(&mut self) -> &mut Option<OutputHandle> {
        &mut self.output_handle
    }

    fn flow_state(&self) -> &Self::FlowState {
        &self.flow_state
    }

    fn unsettled(&self) -> &Self::Unsettled {
        &self.unsettled
    }

    fn rcv_settle_mode(&self) -> &ReceiverSettleMode {
        &self.rcv_settle_mode
    }

    fn target(&self) -> &Option<Self::Target> {
        &self.target
    }

    fn max_message_size(&self) -> Option<u64> {
        match self.max_message_size {
            0 => None,
            _ => Some(self.max_message_size),
        }
    }

    /// # Cancel safety
    ///
    /// This should be cancel safe if oneshot channel is cancel safe
    async fn exchange_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
        session: &mpsc::Sender<SessionControl>,
        is_reattaching: bool,
    ) -> Result<Self::AttachExchange, ReceiverAttachError> {
        // Send out local attach
        self.send_attach(writer, session, is_reattaching).await?; // FIXME: cancel safe? if oneshot channel is cancel safe

        // Wait for remote attach
        let remote_attach = match reader
            .recv()
            .await // cancel safe
            .ok_or(ReceiverAttachError::IllegalSessionState)?
        {
            LinkFrame::Attach(attach) => attach,
            _ => return Err(ReceiverAttachError::NonAttachFrameReceived),
        };

        self.on_incoming_attach(remote_attach)
    }

    async fn handle_attach_error(
        &mut self,
        attach_error: ReceiverAttachError,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
        session: &mpsc::Sender<SessionControl>,
    ) -> ReceiverAttachError {
        match attach_error {
            // Errors that indicate failed attachment
            ReceiverAttachError::IllegalSessionState
            | ReceiverAttachError::IllegalState
            | ReceiverAttachError::NonAttachFrameReceived
            | ReceiverAttachError::ExpectImmediateDetach
            | ReceiverAttachError::RemoteClosedWithError(_) => attach_error,

            ReceiverAttachError::DuplicatedLinkName => {
                let error = definitions::Error::new(
                    SessionError::HandleInUse,
                    "Link name is in use".to_string(),
                    None,
                );
                session
                    .send(SessionControl::End(Some(error)))
                    .await
                    .map(|_| attach_error)
                    .unwrap_or(ReceiverAttachError::IllegalSessionState)
            }

            // ReceiverAttachError::SndSettleModeNotSupported
            ReceiverAttachError::RcvSettleModeNotSupported
            | ReceiverAttachError::IncomingSourceIsNone => {
                // Just send detach immediately
                let err = self
                    .send_detach(writer, true, None)
                    .await
                    .map(|_| attach_error)
                    .unwrap_or(ReceiverAttachError::IllegalSessionState);
                recv_detach(self, reader, err).await
            }

            ReceiverAttachError::CoordinatorIsNotImplemented
            | ReceiverAttachError::InitialDeliveryCountIsNone
            | ReceiverAttachError::SourceAddressIsNoneWhenDynamicIsTrue
            | ReceiverAttachError::TargetAddressIsSomeWhenDynamicIsTrue
            | ReceiverAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse => {
                match (&attach_error).try_into() {
                    Ok(error) => match self.send_detach(writer, true, Some(error)).await {
                        Ok(_) => recv_detach(self, reader, attach_error).await,
                        Err(_) => ReceiverAttachError::IllegalSessionState,
                    },
                    Err(_) => attach_error,
                }
            }
            _ => attach_error,
        }
    }
}

async fn recv_detach<T>(
    link: &mut ReceiverLink<T>,
    reader: &mut mpsc::Receiver<LinkFrame>,
    err: ReceiverAttachError,
) -> ReceiverAttachError
where
    T: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync,
{
    match reader.recv().await {
        Some(LinkFrame::Detach(remote_detach)) => match link.on_incoming_detach(remote_detach) {
            Ok(_) => err,
            Err(detach_error) => detach_error.try_into().unwrap_or(err),
        },
        Some(_) => ReceiverAttachError::NonAttachFrameReceived,
        None => ReceiverAttachError::IllegalSessionState,
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp_types::{
        messaging::{
            message::{Body, __private::Serializable},
            AmqpValue, DeliveryAnnotations, Header, Message, MessageAnnotations,
        },
        primitives::{OrderedMap, Value},
    };
    use serde_amqp::to_vec;

    use crate::link::receiver_link::count_number_of_sections_and_offset;

    use super::is_consecutive;

    #[test]
    fn test_section_numbers() {
        let message = Message {
            header: Some(Header {
                durable: true,
                ..Default::default()
            }),
            // header: None,
            delivery_annotations: Some(DeliveryAnnotations(OrderedMap::new())),
            // delivery_annotations: None,
            message_annotations: Some(MessageAnnotations(OrderedMap::new())),
            // message_annotations: None,
            properties: None,
            application_properties: None,
            body: Body::Value(AmqpValue(Value::Bool(true))),
            footer: None,
        };
        // let mut buf = Vec::new();
        // let mut serializer = serde_amqp::ser::Serializer::new(&mut buf);
        // message.serialize(&mut serializer).unwrap();
        let buf = to_vec(&Serializable(message)).unwrap();
        let (nums, offset) = count_number_of_sections_and_offset(&buf);
        println!("{:?}, {:?}", nums, offset);
    }

    #[test]
    fn test_consecutive_chunks() {
        let expected = vec![vec![0u32, 1, 2, 3], vec![5, 6], vec![8, 9], vec![11]];
        let vals: Vec<u32> = expected.iter().flatten().map(|el| *el).collect();
        assert_eq!(vals.len() - 1, vals.windows(2).len());

        let inds: Vec<usize> = vals
            .windows(2)
            .enumerate()
            .filter_map(|(i, vals)| {
                if is_consecutive(&vals[0], &vals[1]) {
                    None
                } else {
                    Some(i + 1)
                }
            })
            .collect();

        let mut prev_ind = 0;
        for i in 0..inds.len() {
            let ind = inds[i];
            let slice = &vals[prev_ind..ind];
            assert_eq!(slice, expected[i]);
            prev_ind = ind;
        }
        let final_slice = &vals[prev_ind..];
        assert_eq!(final_slice, expected.last().unwrap())
    }
}
