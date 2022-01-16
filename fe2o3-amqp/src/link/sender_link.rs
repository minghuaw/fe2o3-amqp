use super::*;

#[async_trait]
impl endpoint::SenderLink
    for Link<role::Sender, Consumer<Arc<LinkFlowState<role::Sender>>>, UnsettledMessage>
{
    async fn send_transfer<W>(
        &mut self,
        writer: &mut W,
        mut payload: Payload,
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

            // match settled {
            //     true => Ok(Settlement::Settled),
            //     // If not set on the first (or only) transfer for a (multi-transfer)
            //     // delivery, then the settled flag MUST be interpreted as being false.
            //     false => {
            //         let (tx, rx) = oneshot::channel();
            //         let unsettled = UnsettledMessage::new(payload, tx);
            //         {
            //             let mut guard = self.unsettled.write().await;
            //             guard.insert(delivery_tag, unsettled);
            //         }

            //         Ok(Settlement::Unsettled {
            //             delivery_tag: tag,
            //             outcome: rx,
            //         })
            //     }
            // }
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
                more: true, // There are more content
                // If not set, this value is defaulted to the value negotiated
                // on link attach.
                rcv_settle_mode: None,
                state: state.clone(), // This is None for all transfers for now
                resume,
                aborted: false,
                batchable
            };
            send_transfer(writer, transfer, partial).await?;

            // Send the transfers in the middle
            for _ in 1..n-1 {
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
                    batchable
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
                batchable
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
                    delivery_tag: tag,
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