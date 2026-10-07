use std::sync::{Arc, OnceLock};

use fe2o3_amqp_types::definitions::Fields;
use futures_util::Future;
use serde_amqp::serialized_size;

use crate::endpoint::LinkExt;

use super::{resumption::resume_delivery, *};

impl<T> SenderLink<T>
where
    T: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync,
{
    /// Whether a transfer goes out settled, which leaves no outcome to wait for.
    ///
    /// If not set on the first (or only) transfer for a (multi-transfer) delivery, then the
    /// settled flag MUST be interpreted as being false.
    pub(crate) fn is_settled_on_send(&self, transfer: &Transfer) -> bool {
        transfer.settled.unwrap_or(match self.snd_settle_mode {
            SenderSettleMode::Settled => true,
            SenderSettleMode::Unsettled => false,
            SenderSettleMode::Mixed => false,
        })
    }

    /// Hands the transfer of an unsettled delivery to the session, registering the delivery in
    /// the unsettled map first.
    ///
    /// The session engine is a task of its own: it can write the transfer, receive the peer's
    /// settled disposition and look the delivery up in the unsettled map before the sending task
    /// resumes. A disposition that finds no entry is dropped, so a delivery registered after the
    /// hand-over could wait for its outcome forever.
    ///
    /// # Cancel safety
    ///
    /// This is cancel safe: an error, or a future dropped before the session has taken the whole
    /// transfer, withdraws the registration, so the map keeps only deliveries whose transfer was
    /// handed over.
    pub(crate) async fn send_unsettled_transfer(
        &self,
        writer: &mpsc::Sender<LinkFrame>,
        transfer: Transfer,
        payload: Payload,
        delivery_tag: DeliveryTag,
        unsettled: &mut Option<UnsettledMessage>,
    ) -> Result<(), TransferError> {
        self.unsettled
            .write()
            .get_or_insert(OrderedMap::new())
            .insert(
                delivery_tag.clone(),
                unsettled
                    .take()
                    .expect("the caller supplies the unsettled message"),
            );
        let registration = UnsettledRegistration {
            unsettled: &self.unsettled,
            delivery_tag: Some(&delivery_tag),
        };
        match self
            .send_transfer_without_modifying_unsettled_map(writer, transfer, payload)
            .await
        {
            Ok(_) => {
                registration.keep();
                Ok(())
            }
            Err(error) => {
                // Recover the message so the caller can retry it instead of
                // losing its settlement channel. The relay may have removed
                // and failed the entry in the meantime, in which case there is
                // nothing left to recover.
                *unsettled = self
                    .unsettled
                    .write()
                    .as_mut()
                    .and_then(|map| map.swap_remove(&delivery_tag));
                registration.keep();
                Err(error)
            }
        }
    }

    /// # Cancel safety
    ///
    /// This is cancel safe because all internal `.await` are cancel safe
    pub(crate) async fn send_transfer_without_modifying_unsettled_map(
        &self,
        writer: &mpsc::Sender<LinkFrame>,
        mut transfer: Transfer,
        mut payload: Payload,
    ) -> Result<bool, TransferError> {
        let settled = self.is_settled_on_send(&transfer);
        let input_handle = self
            .input_handle
            .clone()
            .ok_or(TransferError::NotAttached)?;

        // The connection engine publishes the negotiated encoder max frame
        // length before the connection handle is created; links are only
        // attached after the connection is opened, so the value is always
        // populated. Per AMQP 1.0 §2.7.1 `max-frame-size` is directional: a
        // peer MUST NOT send frames larger than its partner can accept, so
        // the outbound limit is the remote's advertised value as enforced by
        // the transport encoder (see `ConnectionEngine::max_frame_size`).
        //
        // Maximum payload bytes per transfer frame. The frame body (serialized
        // transfer performative + payload) must fit within the encoder's
        // `max_frame_body_size` (i.e. `self.max_frame_size - 4`): the 4-byte
        // frame header (doff, type, channel) is written by the encoder and the
        // 4-byte size prefix is added by the length-delimited codec, so a
        // frame that respects this bound never exceeds the negotiated max
        // frame size. The delivery-id is only assigned by the session *after*
        // the link splits, so its worst-case size (a uint, 0x70 + 4 bytes) is
        // reserved to guarantee that the first frame still fits. The transfer
        // is measured in place with the worst-case delivery-id set (and
        // restored afterwards) so the real serializer decides the list
        // encoding; `serialized_size` computes the size without allocating a
        // buffer. The first frame of a split delivery is sent with `more=true`
        // (the single-frame branch below explicitly resets it to `false`).
        transfer.more = true;
        let orig_delivery_id = transfer.delivery_id; // None on all send paths
        transfer.delivery_id = Some(u32::MAX);
        let performative_size =
            serialized_size(&transfer).map_err(|source| MessageEncodeError { source })?;
        transfer.delivery_id = orig_delivery_id;
        // Saturate so a pathologically small max-frame-size (one that cannot
        // even fit the serialized performative) reports an error instead of
        // underflowing into a huge payload bound.
        let max_payload = self
            .max_frame_size
            .saturating_sub(4)
            .saturating_sub(performative_size);
        if max_payload == 0 {
            // The serialized performative alone does not fit into a single
            // frame, so the delivery cannot be split at all.
            return Err(TransferError::FrameSizeTooSmall);
        }

        // Split the payload so that every transfer frame fits within the
        // negotiated max frame size, keeping the session's transfer-id and
        // window accounting consistent with the frames actually sent.
        let more = payload.len() > max_payload;
        if !more {
            transfer.more = false;
            send_transfer(
                writer,
                input_handle,
                transfer,
                payload.clone(),
                &self.session_stop_reason,
            )
            .await?;
        // cancel safe
        } else {
            // Send the first frame
            let partial = payload.split_to(max_payload);
            transfer.more = true;
            send_transfer(
                writer,
                input_handle.clone(),
                transfer.clone(),
                partial,
                &self.session_stop_reason,
            )
            .await?; // cancel safe

            // AMQP 1.0 §2.7.5: on continuation transfers the delivery-id may be
            // omitted and the delivery-tag / message-format can only be omitted.
            // Clear them *before* the loop so the last transfer is also cleared
            // even when there is no middle frame (i.e. the delivery spans exactly
            // two frames).
            transfer.delivery_tag = None;
            transfer.message_format = None;
            transfer.settled = None;
            transfer.rcv_settle_mode = None;

            // Send the transfers in the middle
            while payload.len() > max_payload {
                let partial = payload.split_to(max_payload);
                send_transfer(
                    writer,
                    input_handle.clone(),
                    transfer.clone(),
                    partial,
                    &self.session_stop_reason,
                )
                .await?;
                // cancel safe
            }

            // Send the last transfer
            // For messages that are too large to fit within the maximum frame size, additional
            // data MAY be trans- ferred in additional transfer frames by setting the more flag on
            // all but the last transfer frame
            transfer.more = false;
            send_transfer(
                writer,
                input_handle,
                transfer,
                payload,
                &self.session_stop_reason,
            )
            .await?;
            // cancel safe
        }

        Ok(settled)
    }

    /// Wait for the next delivery tag, consuming one unit of link credit, or
    /// fail if the link is detaching or stopping at the same time.
    ///
    /// The incoming frame is polled before the credit so a remote detach that
    /// has already arrived is observed before credit is consumed; otherwise a
    /// send could race the remote closing the link, consuming credit and
    /// writing a transfer the relay can no longer forward. The failure is the
    /// peer's detach/close outcome, an unsupported remote-initiated
    /// acquisition, the session stop reason, or a defensive unexpected-frame
    /// error.
    pub(crate) async fn next_delivery_tag<Fut>(
        &mut self,
        detached: Fut,
    ) -> Result<[u8; 4], TransferError>
    where
        Fut: Future<Output = Option<LinkFrame>> + Send,
    {
        use crate::util::Consume;

        // The detach branch is polled first so that a remote detach that has
        // already arrived is observed before a concurrently granted link-credit
        // is consumed. Otherwise a send could race the remote closing the link,
        // consuming credit and writing a transfer the relay can no longer
        // forward.
        tokio::select! {
            biased;
            frame = detached => { // cancel safe
                match frame {
                    // The relay already sent the reply to this peer detach;
                    // the link records the outcome here. Reported as-is even
                    // when the session is stopping: a stop without a detach
                    // shows up as the channel closing (`None` below).
                    Some(LinkFrame::Detach(detach)) => {
                        match self.apply_remote_detach_outcome(detach) {
                            Ok(status) => Err(TransferError::LinkDetached(status)),
                            Err(err) => Err(TransferError::LinkState(err.into())),
                        }
                    },
                    // A remote-initiated transactional acquisition is not
                    // supported yet; the caller terminates the link with
                    // `amqp:not-implemented` (AMQP 1.0 §4.4.3).
                    #[cfg(feature = "transaction")]
                    Some(LinkFrame::Acquisition(_)) => {
                        Err(TransferError::AcquisitionNotImplemented)
                    }
                    Some(_frame) => {
                        // Other frames should not forwarded to the sender by the session
                        #[cfg(feature = "tracing")]
                        tracing::error!("Unexpected frame: {:?}", _frame);
                        #[cfg(feature = "log")]
                        log::error!("Unexpected frame: {:?}", _frame);

                        Err(TransferError::UnexpectedFrame)
                    }
                    None => {
                        // The channel closed without a frame: the session
                        // stopped, or the peer detached the link and the relay
                        // was removed while the session stayed alive.
                        Err(TransferError::LinkState(link_error_from_closed_channel(
                            &self.session_stop_reason,
                            &self.local_state,
                        )))
                    }
                }
            },
            tag = self.flow_state.consume(1) => {
                // link-credit is defined as
                // "The current maximum number of messages that can be handled
                // at the receiver endpoint of the link"

                // Draining should already set the link credit to 0, causing
                // sender to wait for new link credit
                Ok(tag)
            }
        }
    }

    pub(crate) fn generate_non_resuming_transfer_performative(
        &self,
        delivery_tag: DeliveryTag,
        message_format: MessageFormat,
        settled: Option<bool>,
        state: Option<DeliveryState>,
        batchable: bool,
    ) -> Result<Transfer, TransferError> {
        let handle = self
            .output_handle
            .clone()
            .ok_or(TransferError::NotAttached)?
            .into();

        let settled = match self.snd_settle_mode {
            SenderSettleMode::Settled => true,
            SenderSettleMode::Unsettled => false,
            // If not set on the first (or only) transfer for a (multi-transfer)
            // delivery, then the settled flag MUST be interpreted as being false.
            SenderSettleMode::Mixed => settled.unwrap_or(false),
        };

        // If true, the resume flag indicates that the transfer is being used to reassociate an
        // unsettled delivery from a dissociated link endpoint
        let resume = false;

        let transfer = Transfer {
            handle,
            delivery_id: None, // This will be set by the session
            delivery_tag: Some(delivery_tag),
            message_format: Some(message_format),
            settled: Some(settled),
            more: false, // This will be changed later

            // If not set, this value is defaulted to the value negotiated
            // on link attach.
            rcv_settle_mode: None,
            state,
            resume,
            aborted: false,
            batchable,
        };
        Ok(transfer)
    }
}

impl<T> endpoint::SenderLink for SenderLink<T>
where
    T: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync,
{
    type FlowError = FlowError;
    type TransferError = TransferError;
    type DispositionError = DispositionError;

    async fn send_payload<Fut>(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        detached: Fut,
        payload: Payload,
        message_format: MessageFormat,
        settled: Option<bool>,
        state: Option<DeliveryState>,
        batchable: bool,
    ) -> Result<Settlement, Self::TransferError>
    where
        Fut: Future<Output = Option<LinkFrame>> + Send,
    {
        let tag = self.next_delivery_tag(detached).await?;
        // Delivery count is incremented when consuming credit
        let delivery_tag = DeliveryTag::from(tag);

        let transfer = self.generate_non_resuming_transfer_performative(
            delivery_tag,
            message_format,
            settled,
            state,
            batchable,
        )?;

        self.send_payload_with_transfer(writer, message_format, transfer, payload)
            .await
    }

    /// # Cancel safety
    ///
    /// This is cancel safe because all internal `.await` are cancel safe
    async fn send_payload_with_transfer(
        &self,
        writer: &mpsc::Sender<LinkFrame>,
        message_format: MessageFormat,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Settlement, Self::TransferError> {
        // Keep a copy for unsettled message
        // Clone should be very cheap on Bytes
        let payload_copy = payload.clone();
        let delivery_tag = transfer
            .delivery_tag
            .clone()
            .ok_or(LinkStateError::InvariantViolation)?;
        if self.is_settled_on_send(&transfer) {
            self.send_transfer_without_modifying_unsettled_map(writer, transfer, payload)
                .await?;
            return Ok(Settlement::Settled(delivery_tag));
        }

        let (tx, rx) = oneshot::channel();
        let mut unsettled = Some(UnsettledMessage::new(
            payload_copy,
            None,
            message_format,
            tx,
        ));
        self.send_unsettled_transfer(
            writer,
            transfer,
            payload,
            delivery_tag.clone(),
            &mut unsettled,
        )
        .await?;

        Ok(Settlement::Unsettled {
            delivery_tag,
            outcome: rx,
        })
    }

    async fn dispose(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError> {
        if let SenderSettleMode::Settled = self.snd_settle_mode {
            return Ok(());
        }

        {
            let mut lock = self.unsettled.write();
            if settled {
                if let Some(msg) = lock.as_mut().and_then(|m| m.swap_remove(&delivery_tag)) {
                    let _ = msg.settle();
                }
            } else if let Some(msg) = lock.as_mut().and_then(|m| m.get_mut(&delivery_tag)) {
                msg.state = Some(state.clone());
            }
        }

        send_disposition(
            writer,
            delivery_id,
            None,
            settled,
            Some(state),
            batchable,
            &self.session_stop_reason,
        )
        .await
    }

    async fn batch_dispose(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        mut ids_and_tags: Vec<(DeliveryNumber, DeliveryTag)>,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError> {
        if let SenderSettleMode::Settled = self.snd_settle_mode {
            return Ok(());
        }

        let mut first = None;
        let mut last = None;

        ids_and_tags.sort_by_key(|left| left.0);

        // Find continuous ranges
        for (delivery_id, delivery_tag) in ids_and_tags {
            {
                // Make sure there is not .await point during the lifetime of the guard
                let mut guard = self.unsettled.write();
                if settled {
                    if let Some(msg) = guard.as_mut().and_then(|m| m.swap_remove(&delivery_tag)) {
                        let _ = msg.settle();
                    }
                } else if let Some(msg) = guard.as_mut().and_then(|m| m.get_mut(&delivery_tag)) {
                    msg.state = Some(state.clone());
                }
            }

            match (first, last) {
                // First pair
                (None, _) => first = Some(delivery_id),
                // Second pair
                (Some(first_id), None) => {
                    // Find discontinuity
                    if delivery_id - first_id > 1 {
                        send_disposition(
                            writer,
                            first_id,
                            None,
                            settled,
                            Some(state.clone()),
                            batchable,
                            &self.session_stop_reason,
                        )
                        .await?;
                    }
                    last = Some(delivery_id);
                }
                // Third and more
                (Some(first_id), Some(last_id)) => {
                    // Find discontinuity
                    if delivery_id - last_id > 1 {
                        send_disposition(
                            writer,
                            first_id,
                            Some(last_id),
                            settled,
                            Some(state.clone()),
                            batchable,
                            &self.session_stop_reason,
                        )
                        .await?;
                    }
                    last = Some(delivery_id);
                }
            }
        }

        // if there is only one message to dispose
        if let (Some(first_id), None) = (first, last) {
            send_disposition(
                writer,
                first_id,
                None,
                settled,
                Some(state),
                batchable,
                &self.session_stop_reason,
            )
            .await?;
        }
        Ok(())
    }
}

/// # Cancel safety
///
/// This is cancel safe because it only involves `.await` on sending over `tokio::mpsc::Sender`
#[inline]
async fn send_transfer(
    writer: &mpsc::Sender<LinkFrame>,
    input_handle: InputHandle,
    transfer: Transfer,
    payload: Payload,
    session_stop_reason: &OnceLock<SessionStopped>,
) -> Result<(), LinkStateError> {
    let frame = LinkFrame::Transfer {
        input_handle,
        performative: transfer,
        payload,
    };
    writer
        .send(frame)
        .await // cancel safe
        .map_err(|_| link_state_error_from_stop_reason(session_stop_reason))
}

#[inline]
async fn send_disposition(
    writer: &mpsc::Sender<LinkFrame>,
    first: DeliveryNumber,
    last: Option<DeliveryNumber>,
    settled: bool,
    state: Option<DeliveryState>,
    batchable: bool,
    session_stop_reason: &OnceLock<SessionStopped>,
) -> Result<(), LinkStateError> {
    let disposition = Disposition {
        role: Role::Sender,
        first,
        last,
        settled,
        state,
        batchable,
    };
    let frame = LinkFrame::Disposition(disposition);
    writer
        .send(frame)
        .await
        .map_err(|_| link_state_error_from_stop_reason(session_stop_reason))
}

impl<T> SenderLink<T> {
    #[allow(clippy::needless_collect)]
    pub(crate) fn handle_unsettled_in_attach(
        &mut self,
        remote_unsettled: Option<OrderedMap<DeliveryTag, Option<DeliveryState>>>,
    ) -> Result<SenderAttachExchange, SenderAttachError> {
        let mut guard = self.unsettled.write();
        let v: Vec<(DeliveryTag, ResumingDelivery)> = match (guard.take(), remote_unsettled) {
            (None, None) => return Ok(SenderAttachExchange::Complete),
            (None, Some(remote_map)) => {
                if remote_map.is_empty() {
                    return Ok(SenderAttachExchange::Complete);
                }

                // The peer considers these deliveries unsettled while this side
                // has no record of them (target-only). AMQP 1.0 §2.6.13: the
                // sender MUST ignore them, and §2.7.5 forbids sending resumed
                // transfers for deliveries not in the local unsettled map. The
                // receiver settles them by comparing the attach maps.
                Vec::new()
            }
            (Some(local_map), None) => {
                if local_map.is_empty() {
                    return Ok(SenderAttachExchange::Complete);
                }

                local_map
                    .into_iter()
                    .filter_map(|(tag, local)| {
                        resume_delivery(local, None).map(|resume| (tag, resume))
                    })
                    .collect()
            }
            (Some(local_map), Some(mut remote_map)) => {
                if local_map.is_empty() && remote_map.is_empty() {
                    return Ok(SenderAttachExchange::Complete);
                }

                // `remote_map` retains only target-only deliveries after the
                // local tags are removed. The sender MUST ignore them
                // (AMQP 1.0 §2.6.13) and MUST NOT send resumed transfers for
                // deliveries not in its local unsettled map (§2.7.5).
                local_map
                    .into_iter()
                    .filter_map(|(tag, local)| {
                        let remote = remote_map.swap_remove(&tag);
                        resume_delivery(local, remote).map(|resume| (tag, resume))
                    })
                    .collect()
            }
        };

        match self.local_state {
            LinkState::IncompleteAttachReceived
            | LinkState::IncompleteAttachSent
            | LinkState::IncompleteAttachExchanged => {
                Ok(SenderAttachExchange::IncompleteUnsettled(v))
            }
            _ => Ok(SenderAttachExchange::Resume(v)),
        }
    }
}

impl<T> endpoint::LinkAttach for SenderLink<T>
where
    T: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync,
{
    type AttachExchange = SenderAttachExchange;
    type AttachError = SenderAttachError;

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
            (LinkState::Unattached, false) | (LinkState::Detached(_), false) => {
                self.local_state = LinkState::AttachReceived; // re-attaching
            }
            (LinkState::AttachSent, true) | (LinkState::IncompleteAttachSent, true) => {
                self.local_state = LinkState::IncompleteAttachExchanged;
            }
            (LinkState::Unattached, true) | (LinkState::Detached(_), true) => {
                self.local_state = LinkState::IncompleteAttachReceived; // re-attaching
            }
            _ => return Err(SenderAttachError::IllegalState),
        };

        self.input_handle = Some(InputHandle::from(remote_attach.handle));

        // In this case, the sender is considered to hold the authoritative version of the
        // version of the source properties
        //
        // Only need to check the target
        //
        // If there is no pre-existing terminus, and the peer does not wish to create a new one,
        // this is indicated by setting the local terminus (source or target as appropriate) to null.
        if self.verify_incoming_source {
            if let (Some(local_source), Some(remote_source)) = (&self.source, &remote_attach.source)
            {
                local_source.verify_as_sender(remote_source)?;
            }
        }

        let target = remote_attach
            .target
            .map(|t| T::try_from(*t))
            .transpose()
            .map_err(|_| SenderAttachError::CoordinatorIsNotImplemented)?;

        // Note that it is the responsibility of the transaction controller to
        // verify that the capabilities of the controller meet its requirements.
        //
        // the receiver is considered to hold the authoritative version of the target properties
        match (&self.target, &target) {
            (Some(local_target), Some(remote_target)) => {
                if self.verify_incoming_target {
                    local_target.verify_as_sender(remote_target)?
                }
            }
            // Only need to check the target
            //
            // If there is no pre-existing terminus, and the peer does not wish to create a new one,
            // this is indicated by setting the local terminus (source or target as appropriate) to null.
            (_, None) => return Err(SenderAttachError::IncomingTargetIsNone),
            _ => {}
        }
        self.target = target;

        // The `rcv-settle-mode` field in the attach response from the receiver is the
        // receiver's *actual* settlement mode in use ("when set at the receiver this
        // indicates the actual settlement mode in use"). The sender must adapt its
        // settlement handshake accordingly: with `second`, the receiver settles only
        // after receiving the sender's confirming disposition. The session already
        // propagates the actual mode to the link relay (`SessionInner::on_incoming_attach`),
        // which drives the confirming echo at delivery time, so no strict validation
        // is needed here. The local value is still updated with the receiver's actual
        // mode so that it reflects the negotiated state (e.g. on reattach and through
        // the `rcv_settle_mode` accessor), mirroring how the receiver side records the
        // sender's actual `snd-settle-mode`.
        self.rcv_settle_mode = remote_attach.rcv_settle_mode;

        // The `snd-settle-mode` field in the attach response from the receiver only
        // expresses the receiver's *desired* settlement mode for the sender. When the
        // sender initiates the attach, the sender's own choice is the settlement mode in
        // use and the receiver SHOULD respect it, failing the attach if it cannot.
        //
        // A definite conflict (neither side is `mixed`) is still treated as an error
        // because it signals a receiver that expects a settlement behavior the sender
        // will not provide; a `mixed` response is tolerated either way.
        if let (SenderSettleMode::Settled, SenderSettleMode::Unsettled)
        | (SenderSettleMode::Unsettled, SenderSettleMode::Settled) =
            (&self.snd_settle_mode, &remote_attach.snd_settle_mode)
        {
            return Err(SenderAttachError::SndSettleModeNotSupported);
        }

        self.max_message_size =
            get_max_message_size(self.max_message_size, remote_attach.max_message_size);

        if let Some(remote_properties) = remote_attach.properties {
            self.properties_mut(|local_properties| {
                local_properties
                    .get_or_insert_with(Default::default)
                    .as_inner_mut()
                    .extend(remote_properties.into_inner());
            })
        }

        self.handle_unsettled_in_attach(remote_attach.unsettled)
    }

    async fn send_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        mode: AttachMode,
    ) -> Result<(), Self::AttachError> {
        self.send_attach_inner(writer, mode).await?;
        Ok(())
    }
}

impl<T> endpoint::Link for SenderLink<T> where
    T: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync
{
}

impl<T> endpoint::LinkExt for SenderLink<T>
where
    T: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync,
{
    type FlowState = SenderFlowState;
    type Unsettled = ArcSenderUnsettledMap;
    type Target = T;

    fn local_state(&self) -> &LinkState {
        &self.local_state
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn output_handle_mut(&mut self) -> &mut Option<OutputHandle> {
        &mut self.output_handle
    }

    fn session_stop_reason(&self) -> &Arc<OnceLock<SessionStopped>> {
        &self.session_stop_reason
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

    fn max_message_size(&self) -> Option<u64> {
        match self.max_message_size {
            0 => None,
            _ => Some(self.max_message_size),
        }
    }

    fn properties<F, O>(&self, op: F) -> O
    where
        F: FnOnce(&Option<Fields>) -> O,
    {
        let guard = self.flow_state.state().lock.read();
        op(&guard.properties)
    }

    fn properties_mut<F, O>(&self, op: F) -> O
    where
        F: FnOnce(&mut Option<Fields>) -> O,
    {
        let mut guard = self.flow_state.state().lock.write();
        op(&mut guard.properties)
    }

    async fn exchange_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
        mode: AttachMode,
    ) -> Result<Self::AttachExchange, SenderAttachError> {
        // Send out local attach
        self.send_attach(writer, mode).await?;

        // Wait for remote attach
        let remote_attach = match reader
            .recv()
            .await
            .ok_or_else(|| sender_attach_error_from_stop_reason(&self.session_stop_reason))?
        {
            LinkFrame::Attach(attach) => attach,
            _ => return Err(SenderAttachError::NonAttachFrameReceived),
        };

        self.on_incoming_attach(remote_attach)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn handle_attach_error(
        &mut self,
        attach_error: SenderAttachError,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
        session: &mpsc::Sender<SessionControl>,
    ) -> SenderAttachError {
        match attach_error {
            SenderAttachError::SessionStopped(_)
            | SenderAttachError::SessionNotMapped
            | SenderAttachError::InvariantViolation
            | SenderAttachError::NonAttachFrameReceived
            | SenderAttachError::UnexpectedUnsettledMap
            | SenderAttachError::RemoteClosedWithError(_) => attach_error,

            SenderAttachError::DuplicatedLinkName => {
                let error = definitions::Error::new(
                    SessionError::HandleInUse,
                    "Link name is in use".to_string(),
                    None,
                );
                session
                    .send(SessionControl::End(Some(error)))
                    .await
                    .map(|_| attach_error)
                    .unwrap_or(sender_attach_error_from_stop_reason(
                        &self.session_stop_reason,
                    ))
            }

            SenderAttachError::SndSettleModeNotSupported
            | SenderAttachError::IncomingTargetIsNone => {
                // Just send detach immediately
                match self.send_detach(writer, true, None).await {
                    Ok(()) => recv_detach(self, reader, attach_error).await,
                    Err(detach_error) => sender_detach_failure(detach_error, attach_error),
                }
            }

            SenderAttachError::CoordinatorIsNotImplemented
            | SenderAttachError::IllegalState
            | SenderAttachError::SourceAddressIsSomeWhenDynamicIsTrue
            | SenderAttachError::TargetAddressIsNoneWhenDynamicIsTrue
            | SenderAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse => {
                try_detach_with_error(self, attach_error, writer, reader).await
            }
            #[cfg(feature = "transaction")]
            SenderAttachError::DesireTxnCapabilitiesNotSupported => {
                try_detach_with_error(self, attach_error, writer, reader).await
            }
        }
    }
}

/// Classify a rejected attach whose closing detach could not be sent.
///
/// An already terminal link is not a failure by itself: when the stored
/// outcome carries a remote error the remote error is reported, otherwise the
/// attach failure stays the primary error. A session stop always wins.
fn sender_detach_failure(
    detach_error: DetachError,
    attach_error: SenderAttachError,
) -> SenderAttachError {
    match detach_error {
        DetachError::SessionStopped(reason) => SenderAttachError::SessionStopped(reason),
        DetachError::LinkDetached(status) => match status.remote_error() {
            Some(error) => SenderAttachError::RemoteClosedWithError(error.clone()),
            None => attach_error,
        },
        DetachError::IllegalState | DetachError::InvariantViolation | DetachError::NotAttached => {
            attach_error
        }
    }
}

async fn try_detach_with_error<T>(
    link: &mut SenderLink<T>,
    attach_error: SenderAttachError,
    writer: &mpsc::Sender<LinkFrame>,
    reader: &mut mpsc::Receiver<LinkFrame>,
) -> SenderAttachError
where
    T: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync,
{
    match (&attach_error).try_into() {
        Ok(err) => match link.send_detach(writer, true, Some(err)).await {
            Ok(()) => recv_detach(link, reader, attach_error).await,
            Err(detach_error) => sender_detach_failure(detach_error, attach_error),
        },
        Err(_) => attach_error,
    }
}

async fn recv_detach<T>(
    link: &mut SenderLink<T>,
    reader: &mut mpsc::Receiver<LinkFrame>,
    err: SenderAttachError,
) -> SenderAttachError
where
    T: Into<TargetArchetype>
        + TryFrom<TargetArchetype>
        + VerifyTargetArchetype
        + Clone
        + Send
        + Sync,
{
    match reader.recv().await {
        Some(LinkFrame::Detach(remote_detach)) => match link.on_detach_reply(remote_detach) {
            Ok(status) => match status.remote_error() {
                Some(error) => SenderAttachError::RemoteClosedWithError(error.clone()),
                None => err,
            },
            Err(detach_error) => SenderAttachError::from(detach_error),
        },
        Some(_) => SenderAttachError::NonAttachFrameReceived,
        None => sender_attach_error_from_stop_reason(&link.session_stop_reason),
    }
}

/// Withdraws a delivery registered in the unsettled map unless its whole transfer was handed to
/// the session: on an error, and when the sending future is dropped before the hand-over.
struct UnsettledRegistration<'a> {
    unsettled: &'a ArcSenderUnsettledMap,
    delivery_tag: Option<&'a DeliveryTag>,
}

impl UnsettledRegistration<'_> {
    fn keep(mut self) {
        self.delivery_tag = None;
    }
}

impl Drop for UnsettledRegistration<'_> {
    fn drop(&mut self) {
        if let Some(delivery_tag) = self.delivery_tag.take() {
            if let Some(map) = self.unsettled.write().as_mut() {
                let _ = map.swap_remove(delivery_tag);
            }
        }
    }
}
