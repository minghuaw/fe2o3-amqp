use fe2o3_amqp_types::definitions::SequenceNo;
use futures_util::Future;

use crate::link::error::DetachError;

use super::*;

#[async_trait]
impl endpoint::SenderLink for SenderLink {
    type Error = link::Error;

    /// Set and send flow state
    async fn send_flow<W>(
        &mut self,
        writer: &mut W,
        delivery_count: Option<SequenceNo>,
        available: Option<u32>,
        echo: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
    {
        self.error_if_closed().map_err(|e| link::Error::Local(e))?;

        let handle = self
            .output_handle
            .clone()
            .ok_or_else(|| Error::not_attached())?;

        let flow = match (delivery_count, available) {
            (Some(delivery_count), Some(available)) => {
                let mut writer = self.flow_state.as_ref().lock.write().await;
                writer.delivery_count = delivery_count;
                writer.available = available;
                LinkFlow {
                    handle,
                    delivery_count: Some(delivery_count),
                    // TODO: "last known value"???
                    // The sender endpoint sets this to the last known value seen from the receiver.
                    link_credit: Some(writer.link_credit),
                    available: Some(available),
                    // When flow state is sent from the sender to the receiver, this field
                    // contains the actual drain mode of the sender
                    drain: writer.drain,
                    echo,
                    properties: writer.properties.clone(),
                }
            }
            (Some(delivery_count), None) => {
                let mut writer = self.flow_state.as_ref().lock.write().await;
                writer.delivery_count = delivery_count;
                LinkFlow {
                    handle,
                    delivery_count: Some(delivery_count),
                    // TODO: "last known value"???
                    // The sender endpoint sets this to the last known value seen from the receiver.
                    link_credit: Some(writer.link_credit),
                    available: Some(writer.available),
                    // When flow state is sent from the sender to the receiver, this field
                    // contains the actual drain mode of the sender
                    drain: writer.drain,
                    echo,
                    properties: writer.properties.clone(),
                }
            }
            (None, Some(available)) => {
                let mut writer = self.flow_state.as_ref().lock.write().await;
                writer.available = available;
                LinkFlow {
                    handle,
                    delivery_count: Some(writer.delivery_count),
                    // TODO: "last known value"???
                    // The sender endpoint sets this to the last known value seen from the receiver.
                    link_credit: Some(writer.link_credit),
                    available: Some(available),
                    // When flow state is sent from the sender to the receiver, this field
                    // contains the actual drain mode of the sender
                    drain: writer.drain,
                    echo,
                    properties: writer.properties.clone(),
                }
            }
            (None, None) => {
                let reader = self.flow_state.as_ref().lock.read().await;
                LinkFlow {
                    handle,
                    delivery_count: Some(reader.delivery_count),
                    // TODO: "last known value"???
                    // The sender endpoint sets this to the last known value seen from the receiver.
                    link_credit: Some(reader.link_credit),
                    available: Some(reader.available),
                    // When flow state is sent from the sender to the receiver, this field
                    // contains the actual drain mode of the sender
                    drain: reader.drain,
                    echo,
                    properties: reader.properties.clone(),
                }
            }
        };
        writer
            .send(LinkFrame::Flow(flow))
            .await
            .map_err(|_| Error::sending_to_session())
    }

    async fn send_transfer<W, Fut>(
        &mut self,
        writer: &mut W,
        detached: Fut,
        mut payload: Payload,
        message_format: MessageFormat,
        settled: Option<bool>,
        batchable: bool,
    ) -> Result<Settlement, Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
        Fut: Future<Output = Option<LinkFrame>> + Send,
    {
        use crate::endpoint::LinkDetach;
        use crate::util::Consume;

        self.error_if_closed().map_err(|e| Self::Error::Local(e))?;

        tokio::select! {
            _ = self.flow_state.consume(1) => {
                // link-credit is defined as
                // "The current maximum number of messages that can be handled
                // at the receiver endpoint of the link"

                // Draining should already set the link credit to 0, causing
                // sender to wait for new link credit
            },
            frame = detached => {
                match frame {
                    // If remote has detached the link
                    Some(LinkFrame::Detach(detach)) => {
                        let closed = detach.closed;
                        let result = self.on_incoming_detach(detach).await;
                        self.send_detach(writer, closed, None).await
                            .map_err(|e| Self::Error::Local(e))?;

                        let detach_err = match result {
                            Ok(_) => DetachError {
                                is_closed_by_remote: closed,
                                error: None
                            },
                            Err(err) => DetachError {
                                is_closed_by_remote: closed,
                                error: Some(err)
                            }
                        };

                        return Err(Error::Detached(detach_err))
                    },
                    _ => {
                        // Other frames should not forwarded to the sender by the session
                        return Err(Error::expecting_frame("Detach"))
                    }
                }
            }
        }

        let handle = self
            .output_handle
            .clone()
            .ok_or_else(|| AmqpError::IllegalState)?;

        let tag = self.flow_state.state().delivery_count().await.to_be_bytes();
        let delivery_tag = DeliveryTag::from(tag);

        // TODO: Expose API to allow user to set this when the mode is MIXED?
        let settled = match self.snd_settle_mode {
            SenderSettleMode::Settled => true,
            SenderSettleMode::Unsettled => false,
            // If not set on the first (or only) transfer for a (multi-transfer)
            // delivery, then the settled flag MUST be interpreted as being false.
            SenderSettleMode::Mixed => settled.unwrap_or_else(|| false),
        };

        // TODO: Expose API for resuming link?
        let state: Option<DeliveryState> = None;

        // If true, the resume flag indicates that the transfer is being used to reassociate an
        // unsettled delivery from a dissociated link endpoint
        let resume = false;

        // Keep a copy for unsettled message
        // Clone should be very cheap on Bytes
        let payload_copy = payload.clone();

        // Check message size
        // If this field is zero or unset, there is no maximum size imposed by the link endpoint.
        let more = (self.max_message_size != 0) && (payload.len() as u64 > self.max_message_size);
        // let single_transfer = (self.max_message_size == 0) || (payload.len() as u64 <= self.max_message_size);
        if !more {
            let transfer = Transfer {
                handle,
                delivery_id: None, // This will be set by the session
                delivery_tag: Some(delivery_tag.clone()),
                message_format: Some(message_format),
                settled: Some(settled), // Having this always set in first frame helps debugging
                more: false,
                // If not set, this value is defaulted to the value negotiated
                // on link attach.
                rcv_settle_mode: None,
                state,
                resume,
                aborted: false,
                batchable,
            };

            // TODO: Clone should be very cheap on Bytes
            send_transfer(writer, transfer, payload.clone()).await?;
        } else {
            // Need multiple transfers
            // Number of transfers needed
            let mut n = payload.len() / self.max_message_size as usize;
            if payload.len() > self.max_message_size as usize {
                n += 1
            }

            // Send the first frame
            let partial = payload.split_to(self.max_message_size as usize);
            let transfer = Transfer {
                handle: handle.clone(),
                delivery_id: None, // This will be set by the session
                delivery_tag: Some(delivery_tag.clone()),
                message_format: Some(message_format),
                settled: Some(settled), // Having this always set in first frame helps debugging
                more: true,             // There are more content
                // If not set, this value is defaulted to the value negotiated
                // on link attach.
                rcv_settle_mode: None,
                state: state.clone(), // This is None for all transfers for now
                resume,
                aborted: false,
                batchable,
            };
            send_transfer(writer, transfer, partial).await?;

            // Send the transfers in the middle
            for _ in 1..n - 1 {
                let partial = payload.split_to(self.max_message_size as usize);
                let transfer = Transfer {
                    handle: handle.clone(),
                    delivery_id: None,
                    delivery_tag: None,
                    message_format: None,
                    settled: None,
                    more: true,
                    rcv_settle_mode: None,
                    state: state.clone(), // This is None for all transfers for now
                    resume: false,
                    aborted: false,
                    batchable,
                };
                send_transfer(writer, transfer, partial).await?;
            }

            // Send the last transfer
            // For messages that are too large to fit within the maximum frame size, additional
            // data MAY be trans- ferred in additional transfer frames by setting the more flag on
            // all but the last transfer frame
            let transfer = Transfer {
                handle,
                delivery_id: None,
                delivery_tag: None,
                message_format: None,
                settled: None,
                more: false, // The
                rcv_settle_mode: None,
                state: state.clone(), // This is None for all transfers for now
                resume: false,
                aborted: false,
                batchable,
            };
            send_transfer(writer, transfer, payload).await?;
        }

        match settled {
            true => Ok(Settlement::Settled),
            // If not set on the first (or only) transfer for a (multi-transfer)
            // delivery, then the settled flag MUST be interpreted as being false.
            false => {
                let (tx, rx) = oneshot::channel();
                let unsettled = UnsettledMessage::new(payload_copy, tx);
                {
                    let mut guard = self.unsettled.write().await;
                    guard.insert(delivery_tag, unsettled);
                }

                Ok(Settlement::Unsettled {
                    _delivery_tag: tag,
                    outcome: rx,
                })
            }
        }
    }

    async fn dispose<W>(
        &mut self,
        writer: &mut W,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
    {
        self.error_if_closed().map_err(|e| Error::Local(e))?;
        if let SenderSettleMode::Settled = self.snd_settle_mode {
            return Ok(());
        }

        {
            let mut lock = self.unsettled.write().await;
            if settled {
                if let Some(msg) = lock.remove(&delivery_tag) {
                    let _ = msg.settle();
                }
            } else {
                if let Some(msg) = lock.get_mut(&delivery_tag) {
                    *msg.state_mut() = state.clone();
                }
            }
        }

        send_disposition(writer, delivery_id, None, settled, Some(state), batchable).await
    }

    async fn batch_dispose<W>(
        &mut self,
        writer: &mut W,
        mut ids_and_tags: Vec<(DeliveryNumber, DeliveryTag)>,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
    {
        self.error_if_closed().map_err(|e| Error::Local(e))?;

        if let SenderSettleMode::Settled = self.snd_settle_mode {
            return Ok(());
        }

        let mut first = None;
        let mut last = None;

        // TODO: Is sort necessary?
        ids_and_tags.sort_by(|left, right| left.0.cmp(&right.0));

        let mut lock = self.unsettled.write().await;

        // Find continuous ranges
        for (delivery_id, delivery_tag) in ids_and_tags {
            if settled {
                if let Some(msg) = lock.remove(&delivery_tag) {
                    let _ = msg.settle();
                }
            } else {
                if let Some(msg) = lock.get_mut(&delivery_tag) {
                    *msg.state_mut() = state.clone();
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
                        )
                        .await?;
                    }
                    last = Some(delivery_id);
                }
            }
        }

        // if there is only one message to dispose
        if let (Some(first_id), None) = (first, last) {
            send_disposition(writer, first_id, None, settled, Some(state), batchable).await?;
        }
        Ok(())
    }
}

#[inline]
async fn send_transfer<W>(writer: &mut W, transfer: Transfer, payload: Payload) -> Result<(), Error>
where
    W: Sink<LinkFrame> + Send + Unpin,
{
    let frame = LinkFrame::Transfer {
        performative: transfer,
        payload: payload,
    };
    writer
        .send(frame)
        .await
        .map_err(|_| Error::sending_to_session())
}

#[inline]
async fn send_disposition<W>(
    writer: &mut W,
    first: DeliveryNumber,
    last: Option<DeliveryNumber>,
    settled: bool,
    state: Option<DeliveryState>,
    batchable: bool,
) -> Result<(), Error>
where
    W: Sink<LinkFrame> + Send + Unpin,
{
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
        .map_err(|_| Error::sending_to_session())
}
