use super::*;

#[async_trait]
impl endpoint::SenderLink
    for Link<role::Sender, Consumer<Arc<LinkFlowState<role::Sender>>>, UnsettledMessage>
{
    async fn send_transfer<W>(
        &mut self,
        writer: &mut W,
        payload: Payload,
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

            match settled {
                true => Ok(Settlement::Settled),
                // If not set on the first (or only) transfer for a (multi-transfer)
                // delivery, then the settled flag MUST be interpreted as being false.
                false => {
                    let (tx, rx) = oneshot::channel();
                    let unsettled = UnsettledMessage::new(payload, tx);
                    {
                        let mut guard = self.unsettled.write().await;
                        guard.insert(delivery_tag, unsettled);
                    }

                    Ok(Settlement::Unsettled {
                        delivery_tag: tag,
                        outcome: rx,
                    })
                }
            }
        } else {
            // Need multiple transfers
            // Number of transfers needed
            let mut n = payload.len() / self.max_message_size as usize;
            if payload.len() > self.max_message_size as usize {
                n += 1
            }

            // Send the first frame
            let transfer = Transfer {
                handle,
                delivery_id: None, // This will be set by the session
                delivery_tag: Some(delivery_tag),
                message_format: Some(message_format),
                settled: Some(settled), // Having this always set in first frame helps debugging
                more: true, // There are more content
                // If not set, this value is defaulted to the value negotiated
                // on link attach.
                rcv_settle_mode: None,
                state,
                resume,
                aborted: false,
                batchable
            };


            for i in 0..n {

            }

            todo!()
        }
    }

    async fn dispose<W>(
        &mut self,
        writer: &mut W,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        settled: bool,
        state: DeliveryState,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame, Error = mpsc::error::SendError<LinkFrame>> + Send + Unpin,
    {
        todo!()
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
        .map_err(|_| link::Error::AmqpError {
            condition: AmqpError::IllegalState,
            description: Some("Session is already dropped".to_string()),
        })
}