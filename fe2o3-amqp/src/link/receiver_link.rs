use super::*;

#[async_trait]
impl ReceiverLink for Link<role::Receiver, Arc<LinkFlowState<role::Receiver>>, DeliveryState> {
    async fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Bytes,
    ) -> Result<(Delivery, Option<Disposition>), Self::Error> {
        // TODO: The receiver should then detach with error
        self.flow_state.consume(1).await?;

        // Upon receiving the transfer, the receiving link endpoint (receiver)
        // will create an entry in its own unsettled map and make the transferred
        // message data available to the application to process.

        // This only takes care of whether the message is considered
        // sett
        let settled_by_sender = transfer.settled.unwrap_or_else(|| false);
        let delivery_id = transfer
            .delivery_id
            .ok_or_else(|| Error::AmqpError {
                condition: AmqpError::NotAllowed,
                description: Some("The delivery-id is not found".into()),
            })?;
        let delivery_tag = transfer
            .delivery_tag
            .ok_or_else(|| Error::AmqpError {
                condition: AmqpError::NotAllowed,
                description: Some("The delivery-tag is not found".into()),
            })?;

        let (message, disposition) = if settled_by_sender {
            // If the message is pre-settled, there is no need to
            // add to the unsettled map and no need to reply to the Sender
            (from_reader(payload.reader())?, None)
        } else {
            // If the message is being sent settled by the sender, the value of this
            // field is ignored.
            let mode = if let Some(mode) = &transfer.rcv_settle_mode {
                // If the negotiated link value is first, then it is illegal to set this
                // field to second.
                if let ReceiverSettleMode::First = &self.rcv_settle_mode {
                    if let ReceiverSettleMode::Second = mode {
                        return Err(Error::AmqpError {
                            condition: AmqpError::NotAllowed,
                            description: Some("Negotiated link value is First".into()),
                        });
                    }
                }
                mode
            } else {
                // If not set, this value is defaulted to the value negotiated on link
                // attach.
                &self.rcv_settle_mode
            };

            match mode {
                // If first, this indicates that the receiver MUST settle the delivery
                // once it has arrived without waiting for the sender to settle first.
                ReceiverSettleMode::First => {
                    let message: Message = from_reader(payload.reader())?;
                    // Spontaneously settle the message with an Accept
                    let disposition = Disposition {
                        role: Role::Receiver,
                        first: delivery_id,
                        last: None,
                        settled: true,
                        state: Some(DeliveryState::Accepted(Accepted {})),
                        batchable: false,
                    };
                    (message, Some(disposition))
                }
                // If second, this indicates that the receiver MUST NOT settle until
                // sending its disposition to the sender and receiving a settled
                // disposition from the sender.
                ReceiverSettleMode::Second => {
                    // Add to unsettled map
                    let message: Message = from_reader(payload.reader())?;
                    let state = DeliveryState::Received(Received {
                        section_number: todo!(), // What is section number?
                        section_offset: todo!()
                    });
                    todo!()
                }
            }
        };

        let link_output_handle = self.output_handle.clone().ok_or_else(|| Error::AmqpError {
            condition: AmqpError::IllegalState,
            description: Some("Link is not attached".into()),
        })?;
        let delivery = Delivery {
            link_output_handle,
            delivery_id,
            delivery_tag,
            message,
        };

        Ok((delivery, disposition))
    }
}