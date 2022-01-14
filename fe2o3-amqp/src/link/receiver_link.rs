use serde_amqp::{described::Described, format_code::EncodingCodes};

use super::*;

#[async_trait]
impl ReceiverLink for Link<role::Receiver, Arc<LinkFlowState<role::Receiver>>, DeliveryState> {
    async fn on_incomplete_transfer(
        &mut self,
        delivery_tag: DeliveryTag,
        section_number: u32,
        section_offset: u64,
    ) {
        let state = DeliveryState::Received(Received {
            section_number,
            section_offset,
        });

        {
            let mut lock = self.unsettled.write().await;
            // The same key may be writter multiple times
            let _ = lock.insert(delivery_tag, state);
        }
    }

    async fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: BytesMut,
    ) -> Result<(Delivery, Option<Disposition>), Self::Error> {
        // TODO: The receiver should then detach with error
        self.flow_state.consume(1).await?;

        // Upon receiving the transfer, the receiving link endpoint (receiver)
        // will create an entry in its own unsettled map and make the transferred
        // message data available to the application to process.

        // This only takes care of whether the message is considered
        // sett
        let settled_by_sender = transfer.settled.unwrap_or_else(|| false);
        let delivery_id = transfer.delivery_id.ok_or_else(|| Error::AmqpError {
            condition: AmqpError::NotAllowed,
            description: Some("The delivery-id is not found".into()),
        })?;
        let delivery_tag = transfer.delivery_tag.ok_or_else(|| Error::AmqpError {
            condition: AmqpError::NotAllowed,
            description: Some("The delivery-tag is not found".into()),
        })?;

        let message: Message = from_reader(payload.reader())?;
        let disposition = if settled_by_sender {
            // If the message is pre-settled, there is no need to
            // add to the unsettled map and no need to reply to the Sender
            None
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
                    // Spontaneously settle the message with an Accept
                    let disposition = Disposition {
                        role: Role::Receiver,
                        first: delivery_id,
                        last: None,
                        settled: true,
                        state: Some(DeliveryState::Accepted(Accepted {})),
                        batchable: false,
                    };
                    Some(disposition)
                }
                // If second, this indicates that the receiver MUST NOT settle until
                // sending its disposition to the sender and receiving a settled
                // disposition from the sender.
                ReceiverSettleMode::Second => {
                    // Add to unsettled map
                    // let message: Message = from_reader(payload.reader())?;
                    let state = DeliveryState::Received(Received {
                        section_number: todo!(), // What is section number?
                        section_offset: todo!(),
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

/// Count number of sections in encoded message
pub(crate) fn section_number_and_offset(bytes: &[u8]) -> (u32, u64) {
    const DESCRIBED_TYPE: u8 = EncodingCodes::DescribedType as u8;
    const SMALL_ULONG_TYPE: u8 = EncodingCodes::SmallUlong as u8;
    const ULONG_TYPE: u8 = EncodingCodes::ULong as u8;
    const HEADER_CODE: u8 = 0x70;
    const DELIV_ANNOT_CODE: u8 = 0x71;
    const MSG_ANNOT_CODE: u8 = 0x72;
    const PROP_CODE: u8 = 0x73;
    const APP_PROP_CODE: u8 = 0x74;
    const DATA_CODE: u8 = 0x75;
    const AMQP_SEQ_CODE: u8 = 0x76;
    const AMQP_VAL_CODE: u8 = 0x77;
    const FOOTER_CODE: u8 = 0x78;

    let mut last_pos = 0;
    let mut section_numbers = 0;
    let mut offset = 0;

    let iter = bytes
        .iter()
        .zip(bytes.iter().skip(1).zip(bytes.iter().skip(2)));
    for (i, (b0, (b1, b2))) in iter.enumerate() {
        match (*b0, *b1, *b2) {
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
            | (DESCRIBED_TYPE, ULONG_TYPE, FOOTER_CODE) => {
                section_numbers += 1;
                last_pos = i;
            },
            _ => {}
        }
    }

    offset = bytes.len() - last_pos;

    (section_numbers, offset as u64)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bytes::Bytes;
    use fe2o3_amqp_types::{messaging::{Message, Header, DeliveryAnnotations, MessageAnnotations, message::BodySection, AmqpValue}, primitives::Value};
    use serde_amqp::to_vec;

    use crate::link::receiver_link::section_number_and_offset;

    #[test]
    fn test_iter_u8() {
        let bytes = Bytes::from_static(&[0, 1, 2, 3, 4, 5]);
        let slice: &[u8] = bytes.as_ref();
        let iter = slice.iter().zip(slice.iter().skip(1));
        for item in iter {
            println!("{:?}", item);
        }
    }

    #[test]
    fn test_section_numbers() {
        let message = Message {
            header: Some(Header {
                durable: true,
                ..Default::default()
            }),
            // header: None,
            delivery_annotations: Some(DeliveryAnnotations(BTreeMap::new())),
            // delivery_annotations: None,
            message_annotations: Some(MessageAnnotations(BTreeMap::new())),
            // message_annotations: None,
            properties: None,
            application_properties: None,
            body_section: BodySection::Value(AmqpValue(Value::Bool(true))),
            footer: None,
        };
        let serialized = to_vec(&message).unwrap();
        let (nums, offset) = section_number_and_offset(&serialized);
        println!("{:?}, {:?}", nums, offset);
    }
}
