use fe2o3_amqp_types::messaging::{DeliveryState, Received};

use crate::Payload;

use super::{delivery::UnsettledMessage, receiver_link::is_section_header, SendError};

pub(crate) struct ResumingDelivery {
    resume: bool,
    state: Option<DeliveryState>,
    payload: Payload,
}

fn resume_delivery(
    local: UnsettledMessage,
    remote: Option<DeliveryState>,
) -> Result<Option<ResumingDelivery>, SendError> {
    let outcome = match (&local.state(), &remote) {
        // Illegal delivery states?
        (_, Some(DeliveryState::Declared(_)))
        | (_, Some(DeliveryState::TransactionalState(_)))
        | (Some(DeliveryState::Declared(_)), _)
        | (Some(DeliveryState::TransactionalState(_)), _) => {
            return Err(SendError::IllegalDeliveryState)
        } // TODO: IllegalDeliveryState?

        // delivery-tag 1 example
        (None, None) => {
            Some(ResumingDelivery {
                resume: false,
                state: None,
                payload: local.payload().clone(), // cloning `Bytes` is cheap
            })
        }

        // delivery-tag 2 example
        (
            None,
            Some(DeliveryState::Received(Received {
                section_number,
                section_offset,
            })),
        ) => {
            let remaining = split_off_at_section_and_offset(
                local.payload(),
                *section_number as usize,
                *section_offset as usize,
            )
            .unwrap_or(local.payload().clone());
            Some(ResumingDelivery {
                resume: true,
                state: Some(DeliveryState::Received(Received {
                    section_number: *section_number,
                    section_offset: *section_offset,
                })),
                payload: remaining,
            })
        }

        // delivery-tag 3 example
        (None, Some(DeliveryState::Accepted(_)))
        | (None, Some(DeliveryState::Modified(_)))
        | (None, Some(DeliveryState::Rejected(_)))
        | (None, Some(DeliveryState::Released(_))) => {
            // This will fail if the oneshot receiver is already dropped
            // which means the application probably doesn't care about the 
            // delivery state anyway
            let _ = local.settle_with_state(remote);
            tracing::error!(error = "Delivery handles are already dropped");
            None
        },

        // delivery-tag 5 example
        (Some(DeliveryState::Received(_)), None) => {
            Some(ResumingDelivery {
                resume: false,
                state: None,
                payload: local.payload().clone(),
            })
        },

        // delivery-tag 10 example
        (Some(DeliveryState::Accepted(_)), None)
        | (Some(DeliveryState::Modified(_)), None)
        | (Some(DeliveryState::Rejected(_)), None)
        | (Some(DeliveryState::Released(_)), None) => todo!(),

        // delivery-tag 6 and 7 examples
        (Some(DeliveryState::Received(_)), Some(DeliveryState::Received(_))) => todo!(),

        // delivery-tag 11 example
        (Some(DeliveryState::Accepted(_)), Some(DeliveryState::Received(_)))
        | (Some(DeliveryState::Modified(_)), Some(DeliveryState::Received(_)))
        | (Some(DeliveryState::Rejected(_)), Some(DeliveryState::Received(_)))
        | (Some(DeliveryState::Released(_)), Some(DeliveryState::Received(_))) => todo!(),

        // delivery-tag 8 example
        (Some(DeliveryState::Received(_)), Some(DeliveryState::Accepted(_)))
        | (Some(DeliveryState::Received(_)), Some(DeliveryState::Modified(_)))
        | (Some(DeliveryState::Received(_)), Some(DeliveryState::Rejected(_)))
        | (Some(DeliveryState::Received(_)), Some(DeliveryState::Released(_))) => todo!(),

        // local_state is terminal, remote state is terminal
        // delivery-tag 13 example
        (Some(local_state), Some(remote_state)) => todo!(),
    };
    Ok(outcome)
}

fn split_off_at_section_and_offset(
    payload: &Payload,
    section: usize,
    offset: usize,
) -> Option<Payload> {
    let b0 = payload.iter();
    let b1 = payload.iter().skip(1);
    let b2 = payload.iter().skip(2);
    let zip = b0.zip(b1.zip(b2));

    let mut section_counter = None;
    let mut last_section_index = 0;

    for (i, (&b0, (&b1, &b2))) in zip.enumerate() {
        if is_section_header(b0, b1, b2) {
            match &mut section_counter {
                Some(value) => *value += 1,
                None => section_counter = Some(0),
            }
            last_section_index = i;
        }

        if section_counter == Some(section) && i - last_section_index == offset {
            return Some(payload.slice(i..));
        }
    }

    None
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_zipped_iter() {
        let src = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let b0 = src.iter();
        let b1 = src.iter().skip(1);
        let b2 = src.iter().skip(2);
        let zip = b0.zip(b1.zip(b2));

        for (i, (b0, (b1, b2))) in zip.enumerate() {
            assert_eq!(b0, &src[i]);
            assert_eq!(b1, &src[i + 1]);
            assert_eq!(b2, &src[i + 2]);
        }
    }
}
