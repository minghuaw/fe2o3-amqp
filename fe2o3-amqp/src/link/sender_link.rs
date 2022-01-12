use super::*;

#[async_trait]
impl endpoint::SenderLink
    for Link<role::Sender, Consumer<Arc<LinkFlowState<role::Sender>>>, UnsettledMessage>
{
    async fn send_transfer<W>(
        &mut self,
        writer: &mut W,
        payload: Bytes,
        message_format: MessageFormat,
        settled: Option<bool>,
        batchable: bool,
    ) -> Result<Settlement, <Self as endpoint::Link>::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
    {
        use crate::util::Consume;

        println!(">>> Debug: SenderLink::send_transfer");

        match self.flow_state.consume(1).await {
            SenderPermit::Send => {} // There is enough credit to send
            SenderPermit::Drain => {
                // Drain is set
                todo!()
            }
        }

        // Check message size
        // If this field is zero or unset, there is no maximum size imposed by the link endpoint.
        if (self.max_message_size == 0) || (payload.len() as u64 <= self.max_message_size) {
            let handle = self
                .output_handle
                .clone()
                .ok_or_else(|| AmqpError::IllegalState)?;

            let delivery_tag = self.flow_state.state().delivery_count().await.to_be_bytes();

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
            let resume = false;

            let transfer = Transfer {
                handle,
                delivery_id: None, // This will be set by the session
                delivery_tag: Some(DeliveryTag::from(delivery_tag)),
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

            let frame = LinkFrame::Transfer {
                performative: transfer,
                payload: payload.clone(), // Clone should be very cheap for Bytes
            };
            writer
                .send(frame)
                .await
                .map_err(|_| link::Error::AmqpError {
                    condition: AmqpError::IllegalState,
                    description: Some("Session is already dropped".to_string()),
                })?;

            match settled {
                true => Ok(Settlement::Settled),
                // If not set on the first (or only) transfer for a (multi-transfer)
                // delivery, then the settled flag MUST be interpreted as being false.
                false => {
                    let (tx, rx) = oneshot::channel();
                    let unsettled = UnsettledMessage::new(payload, tx);
                    {
                        let mut guard = self.unsettled.write().await;
                        guard.insert(DeliveryTag::from(delivery_tag), unsettled);
                    }

                    Ok(Settlement::Unsettled {
                        delivery_tag,
                        outcome: rx,
                    })
                }
            }
        } else {
            // Need multiple transfers
            todo!()
        }
    }
}
