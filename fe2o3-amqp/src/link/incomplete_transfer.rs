use fe2o3_amqp_types::performatives::Transfer;

use crate::{Payload, util::AsByteIterator};

use super::{ReceiverTransferError, receiver_link::{count_number_of_sections_and_offset, is_section_header}};

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
    pub buffer: Vec<Payload>,
    pub section_number: Option<u32>,
    pub section_offset: u64,
}

impl IncompleteTransfer {
    pub fn new(transfer: Transfer, partial_payload: Payload) -> Self {
        let (number, offset) = count_number_of_sections_and_offset(&partial_payload);
        Self {
            performative: transfer,
            buffer: vec![partial_payload], // TODO: handle payload split across re-attachment
            section_number: Some(number),
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
        // Count section numbers
        let (number, offset) = count_number_of_sections_and_offset(&other);
        match (&mut self.section_number, number) {
            (_, 0) => self.section_offset += offset,
            (None, 1) => {
                // The first section
                self.section_number = Some(0);
                self.section_offset = offset;
            }
            (None, _) => {
                self.section_number = Some(number - 1);
                self.section_offset = offset;
            }
            (Some(val), _) => {
                *val += number;
                self.section_offset = offset;
            }
        }

        self.buffer.push(other);
    }

    fn position_of_section_number_and_offset(&self, section_number: u32, section_offset: u64) -> Option<usize> {
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
                return Some(i)
            }
        }

        None
    }

    pub fn keep_buffer_till_section_number_and_offset(&mut self, section_number: u32, section_offset: u64) {
        match self.position_of_section_number_and_offset(section_number, section_offset) {
            Some(mut index) => {
                for chunk in self.buffer.iter_mut() {
                    if chunk.len() < index {
                        index -= chunk.len();
                    } else {
                        // Found the chunk
                        let _ = chunk.split_off(index);
                    }
                }
            },
            None => {
                // TODO: should this be treated as an error?
            },
        }
    }
}
