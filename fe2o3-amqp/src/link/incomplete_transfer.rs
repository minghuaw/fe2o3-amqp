use fe2o3_amqp_types::{
    definitions::{DeliveryNumber, DeliveryTag},
    performatives::Transfer,
};

use crate::{util::AsByteIterator, Payload};

use super::{
    receiver_link::{count_number_of_sections_and_offset, is_section_header},
    ReceiverTransferError,
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
    performative: Transfer,
    buffer: Vec<Payload>,
    section_number: u32,
    section_offset: u64,
    /// Sum of the lengths of the chunks in `buffer`, kept in sync so that
    /// max-message-size enforcement does not have to re-sum the buffer for
    /// every incoming transfer frame.
    accumulated_payload_size: u64,
}

impl IncompleteTransfer {
    /// Start buffering a multi-transfer delivery from its first transfer.
    ///
    /// The delivery-id and delivery-tag MUST be specified on the first transfer
    /// of a multi-transfer delivery (AMQP 1.0 §2.7.5). Without them a later
    /// continuation could silently supply them and a truncated delivery could
    /// be assembled.
    pub fn start(
        transfer: Transfer,
        partial_payload: Payload,
    ) -> Result<Self, ReceiverTransferError> {
        if transfer.delivery_id.is_none() {
            return Err(ReceiverTransferError::DeliveryIdIsNone);
        }
        if transfer.delivery_tag.is_none() {
            return Err(ReceiverTransferError::DeliveryTagIsNone);
        }

        let (number, offset) = count_number_of_sections_and_offset(&partial_payload);
        let accumulated_payload_size = partial_payload.len() as u64;
        Ok(Self {
            performative: transfer,
            buffer: vec![partial_payload], // TODO: handle payload split across re-attachment
            section_number: number,
            section_offset: offset,
            accumulated_payload_size,
        })
    }

    /// Append a continuation transfer to this delivery.
    ///
    /// Only a transfer of this delivery is accepted: a repeated or omitted
    /// delivery tag continues the delivery, while an explicitly different tag
    /// is rejected. A present delivery-id or message-format that differs from
    /// the first transfer is rejected as well.
    pub fn try_append(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<(), ReceiverTransferError> {
        if !self.is_same_delivery_as(&transfer) {
            return Err(ReceiverTransferError::InconsistentFieldInMultiFrameDelivery);
        }
        self.or_assign(transfer)?;
        self.append(payload);
        Ok(())
    }

    /// Like `|=` operator but works on the field level
    fn or_assign(&mut self, other: Transfer) -> Result<(), ReceiverTransferError> {
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

    /// Whether `transfer` continues this delivery: it repeats the tag or omits it
    /// (AMQP 1.0 §2.7.5). Only an explicitly different delivery tag is rejected.
    pub fn is_same_delivery_as(&self, transfer: &Transfer) -> bool {
        match (&self.performative.delivery_tag, &transfer.delivery_tag) {
            (Some(local), Some(remote)) => local == remote,
            _ => true,
        }
    }

    pub fn delivery_id(&self) -> Option<DeliveryNumber> {
        self.performative.delivery_id
    }

    /// The delivery tag of the buffered delivery.
    ///
    /// `start` requires a delivery tag on the first transfer and continuation
    /// merges never clear it, so it is always present.
    pub fn delivery_tag(&self) -> &DeliveryTag {
        self.performative
            .delivery_tag
            .as_ref()
            .expect("the buffered delivery always has a delivery tag")
    }

    pub fn section_number(&self) -> u32 {
        self.section_number
    }

    pub fn section_offset(&self) -> u64 {
        self.section_offset
    }

    pub fn accumulated_payload_size(&self) -> u64 {
        self.accumulated_payload_size
    }

    /// Consume the buffered transfer for assembly, yielding the merged
    /// performative, the buffered payload and the section position.
    pub fn into_parts(self) -> (Transfer, Vec<Payload>, u32, u64) {
        (
            self.performative,
            self.buffer,
            self.section_number,
            self.section_offset,
        )
    }

    /// Append to the buffered payload
    fn append(&mut self, other: Payload) {
        // Count section numbers
        let (number, offset) = count_number_of_sections_and_offset(&other);
        if number == 0 {
            self.section_offset += offset;
        } else {
            self.section_number += number;
            self.section_offset = offset;
        }

        self.accumulated_payload_size += other.len() as u64;
        self.buffer.push(other);
    }

    fn position_of_section_number_and_offset(
        &self,
        section_number: u32,
        section_offset: u64,
    ) -> Option<usize> {
        let b0 = self.buffer.as_byte_iterator();
        let b1 = self.buffer.as_byte_iterator().skip(1);
        let b2 = self.buffer.as_byte_iterator().skip(2);
        let iter = b0.zip(b1.zip(b2));

        let mut cur_number = 0;
        let mut cur_offset = 0;

        for (i, (&b0, (&b1, &b2))) in iter.enumerate() {
            cur_offset += 1;

            if is_section_header(b0, b1, b2) {
                cur_number += 1;
                cur_offset = 0;
            }

            if cur_number == section_number && cur_offset == section_offset {
                return Some(i);
            }
        }

        None
    }

    pub fn keep_buffer_till_section_number_and_offset(
        &mut self,
        section_number: u32,
        section_offset: u64,
    ) {
        if let Some(mut index) =
            self.position_of_section_number_and_offset(section_number, section_offset)
        {
            for chunk in self.buffer.iter_mut() {
                if chunk.len() < index {
                    index -= chunk.len();
                } else {
                    // Found the chunk and split the chunk
                    let _ = chunk.split_off(index);
                }
            }

            // The buffer was truncated, so recompute the cached size.
            self.accumulated_payload_size = self.buffer.iter().map(|p| p.len() as u64).sum();
        }
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp_types::{
        definitions::{DeliveryTag, Handle},
        messaging::{message::__private::Serializable, AmqpValue, Body, Header, Message},
        primitives::Value,
    };
    use serde_amqp::to_vec;

    use super::*;

    fn test_transfer(more: bool) -> Transfer {
        Transfer {
            handle: Handle(0),
            delivery_id: Some(0),
            delivery_tag: Some(DeliveryTag::from(vec![0x01])),
            message_format: Some(0),
            settled: Some(false),
            more,
            rcv_settle_mode: None,
            state: None,
            resume: false,
            aborted: false,
            batchable: false,
        }
    }

    /// An encoded message with a header and a body section, so that
    /// section numbering can locate the body (`section_number == 2`).
    fn encoded_message() -> Payload {
        let message = Message {
            header: Some(Header {
                durable: true,
                ..Default::default()
            }),
            delivery_annotations: None,
            message_annotations: None,
            properties: None,
            application_properties: None,
            body: Body::Value(AmqpValue(Value::Bool(true))),
            footer: None,
        };
        Payload::from(to_vec(&Serializable(message)).unwrap())
    }

    #[test]
    fn start_rejects_missing_delivery_id_and_tag() {
        let mut missing_id = test_transfer(true);
        missing_id.delivery_id = None;
        assert!(matches!(
            IncompleteTransfer::start(missing_id, Payload::from(vec![0u8; 4])),
            Err(ReceiverTransferError::DeliveryIdIsNone)
        ));

        let mut missing_tag = test_transfer(true);
        missing_tag.delivery_tag = None;
        assert!(matches!(
            IncompleteTransfer::start(missing_tag, Payload::from(vec![0u8; 4])),
            Err(ReceiverTransferError::DeliveryTagIsNone)
        ));
    }

    #[test]
    fn try_append_rejects_a_different_delivery() {
        let mut incomplete =
            IncompleteTransfer::start(test_transfer(true), encoded_message()).unwrap();

        // A different tag is a different delivery.
        let mut other_tag = test_transfer(true);
        other_tag.delivery_tag = Some(DeliveryTag::from(vec![0x02]));
        assert!(matches!(
            incomplete.try_append(other_tag, Payload::from(vec![0u8; 4])),
            Err(ReceiverTransferError::InconsistentFieldInMultiFrameDelivery)
        ));

        // A different delivery-id is rejected even if the tag repeats.
        let mut other_id = test_transfer(true);
        other_id.delivery_id = Some(1);
        assert!(matches!(
            incomplete.try_append(other_id, Payload::from(vec![0u8; 4])),
            Err(ReceiverTransferError::InconsistentFieldInMultiFrameDelivery)
        ));

        // A different present message-format is rejected as well.
        let mut other_format = test_transfer(true);
        other_format.message_format = Some(1);
        assert!(matches!(
            incomplete.try_append(other_format, Payload::from(vec![0u8; 4])),
            Err(ReceiverTransferError::InconsistentFieldInMultiFrameDelivery)
        ));
    }

    #[test]
    fn try_append_merges_omitted_fields() {
        let mut incomplete =
            IncompleteTransfer::start(test_transfer(true), encoded_message()).unwrap();
        let len = incomplete.accumulated_payload_size;

        let mut continuation = test_transfer(true);
        continuation.delivery_id = None;
        continuation.delivery_tag = None;
        continuation.message_format = None;
        incomplete
            .try_append(continuation, Payload::from(vec![0u8; 17]))
            .unwrap();

        assert_eq!(incomplete.accumulated_payload_size, len + 17);
        assert_eq!(incomplete.delivery_id(), Some(0));
        assert_eq!(incomplete.delivery_tag(), &DeliveryTag::from(vec![0x01]));
    }

    #[test]
    fn append_tracks_accumulated_payload_size() {
        let payload = encoded_message();
        let len = payload.len() as u64;

        let mut incomplete = IncompleteTransfer::start(test_transfer(true), payload).unwrap();
        assert_eq!(incomplete.accumulated_payload_size, len);

        let extra = Payload::from(vec![0u8; 17]);
        incomplete.append(extra);
        assert_eq!(incomplete.accumulated_payload_size, len + 17);
    }

    #[test]
    fn keep_buffer_updates_accumulated_payload_size() {
        let payload = encoded_message();
        let split = payload.len() / 2;
        let first = payload.slice(..split);
        let second = payload.slice(split..);

        let mut incomplete = IncompleteTransfer::start(test_transfer(true), first).unwrap();
        incomplete.append(second);

        // The body section descriptor must be present for the truncation to
        // actually run.
        assert!(incomplete
            .position_of_section_number_and_offset(2, 0)
            .is_some());
        incomplete.keep_buffer_till_section_number_and_offset(2, 0);

        let actual: u64 = incomplete.buffer.iter().map(|p| p.len() as u64).sum();
        assert_eq!(incomplete.accumulated_payload_size, actual);
    }

    #[test]
    fn is_same_delivery_as_accepts_same_or_omitted_tag_and_rejects_other() {
        let incomplete = IncompleteTransfer::start(test_transfer(true), encoded_message()).unwrap();

        let same_tag = test_transfer(true);
        assert!(incomplete.is_same_delivery_as(&same_tag));

        let mut omitted_tag = test_transfer(true);
        omitted_tag.delivery_tag = None;
        assert!(incomplete.is_same_delivery_as(&omitted_tag));

        let mut other_tag = test_transfer(true);
        other_tag.delivery_tag = Some(DeliveryTag::from(vec![0x02]));
        assert!(!incomplete.is_same_delivery_as(&other_tag));
    }
}
