use fe2o3_amqp_types::messaging::{DeliveryState, Received};

use crate::Payload;

use super::{delivery::UnsettledMessage, receiver_link::is_section_header};

pub(crate) enum ResumingDelivery {
    Abort,
    Resend(Payload),
    Resume {
        state: Option<DeliveryState>,
        payload: Payload,
    },
    RestateOutcome {
        local_state: DeliveryState,
    },
}

pub(crate) fn resume_delivery(
    local: UnsettledMessage,
    remote: Option<Option<DeliveryState>>,
) -> Option<ResumingDelivery> {
    // The outer None indicates absence of entry
    let remote = remote.map(|inner| {
        // The inner None indicates absence of DeliveryState, which is equivalent to
        // no recorded data
        inner.unwrap_or(DeliveryState::Received(Received {
            section_number: 0,
            section_offset: 0,
        }))
    });
    match (&local.state(), &remote) {
        #[cfg(feature = "transaction")]
        (_, Some(DeliveryState::Declared(_)))
        | (_, Some(DeliveryState::TransactionalState(_)))
        | (Some(DeliveryState::Declared(_)), _)
        | (Some(DeliveryState::TransactionalState(_)), _) => {
            // Illegal delivery states?
            Some(ResumingDelivery::Abort)
        }

        // delivery-tag 1 example
        (None, None) => Some(ResumingDelivery::Resend(local.payload().clone())),

        // delivery-tag 2 and 4 example
        //
        // delivery-tag 4 has a null in the remote value, which is equivalent to (0,0) Unlike the
        // case with delivery-tag 1 the resent delivery MUST be sent with the resume flag set to
        // true and the delivery-tag set to 4
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
            .unwrap_or_else(|| local.payload().clone());
            Some(ResumingDelivery::Resume {
                state: remote,
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
        }

        // delivery-tag 5 example
        (Some(DeliveryState::Received(_)), None) => {
            Some(ResumingDelivery::Resend(local.payload().clone()))
        }

        // delivery-tag 6, 7 and 9 examples
        (
            Some(DeliveryState::Received(local_recved)),
            Some(DeliveryState::Received(remote_recved)),
        ) => {
            if local_recved <= remote_recved {
                // delivery-tag 6 case
                let remaining = split_off_at_section_and_offset(
                    local.payload(),
                    remote_recved.section_number as usize,
                    remote_recved.section_offset as usize,
                )
                .unwrap_or_else(|| local.payload().clone());
                Some(ResumingDelivery::Resume {
                    state: remote,
                    payload: remaining,
                })
            } else {
                // delivery-tag 7 and 9 case
                //
                // delivery-tag 9 has a null in the remote value,
                // which is equivalent to (0, 0)
                Some(ResumingDelivery::Abort)
            }
        }

        // delivery-tag 8 example
        //
        // This is just like case 3
        (Some(DeliveryState::Received(_)), Some(DeliveryState::Accepted(_)))
        | (Some(DeliveryState::Received(_)), Some(DeliveryState::Modified(_)))
        | (Some(DeliveryState::Received(_)), Some(DeliveryState::Rejected(_)))
        | (Some(DeliveryState::Received(_)), Some(DeliveryState::Released(_))) => {
            // This will fail if the oneshot receiver is already dropped
            // which means the application probably doesn't care about the
            // delivery state anyway
            let _ = local.settle_with_state(remote);
            tracing::error!(error = "Delivery handles are already dropped");
            None
        }

        // delivery-tag 10 example
        //
        // For delivery-tag 10 the receiver has no record of the delivery. However, in contrast to
        // the cases of delivery-tag 1 and delivery-tag 5, since it is known that the sender can
        // only have arrived at this state through knowing that the receiver has received the whole
        // message (or that the sender had spontaneously reached a terminal outcome with no
        // possibility of resumption) there is no need to resend the message
        (Some(DeliveryState::Accepted(_)), None)
        | (Some(DeliveryState::Modified(_)), None)
        | (Some(DeliveryState::Rejected(_)), None)
        | (Some(DeliveryState::Released(_)), None) => {
            let _ = local.settle();
            None
        }

        // delivery-tag 11 and 14 case
        //
        // For delivery-tag 11 it MUST be assumed that the sender spontaneously attained the
        // terminal outcome (and is unable to resume). In this case the sender can simply abort the
        // delivery as it cannot be resumed.
        //
        // For delivery-tag 14 the case is essentially the same as for delivery-tag 11, as the null
        // state at the receiver is essentially identical to having the state Received with
        // section-number=0 and section-offset=0.
        (Some(DeliveryState::Accepted(_)), Some(DeliveryState::Received(_)))
        | (Some(DeliveryState::Modified(_)), Some(DeliveryState::Received(_)))
        | (Some(DeliveryState::Rejected(_)), Some(DeliveryState::Received(_)))
        | (Some(DeliveryState::Released(_)), Some(DeliveryState::Received(_))) => {
            Some(ResumingDelivery::Abort)
        }

        // delivery-tag 12 case
        //
        // For delivery-tag 12 both the sender and receiver have attained the same view of the
        // terminal outcome, but neither has settled. In this case the sender SHOULD simply settle
        // the delivery.
        (Some(DeliveryState::Accepted(_)), Some(DeliveryState::Accepted(_)))
        | (Some(DeliveryState::Modified(_)), Some(DeliveryState::Modified(_)))
        | (Some(DeliveryState::Rejected(_)), Some(DeliveryState::Rejected(_)))
        | (Some(DeliveryState::Released(_)), Some(DeliveryState::Released(_))) => {
            let _ = local.settle();
            None
        }

        // delivery-tag 13 example
        //
        // For delivery-tag 13 the sender and receiver have both attained terminal outcomes, but the
        // outcomes differ. In this case, since the outcome actually takes effect at the sender, it
        // is the sender’s view that is definitive. The sender thus MUST restate this as the
        // terminal outcome, and the receiver SHOULD then echo this and settle.
        (Some(local_state), Some(_)) => Some(ResumingDelivery::RestateOutcome {
            local_state: local_state.clone(),
        }),
    }
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

// enum Expecting {
//     Resend,
//     Resume,
//     Restate,
//     Abort,
//     SettleLocally,
// }

// impl<T> ReceiverLink<T> {

//     // pub(crate) async fn total_expected_resumption(
//     //     &mut self,

//     // )

//     pub(crate) async fn expected_resumption(
//         &mut self,
//         delivery_tag: DeliveryTag,
//         sender: &Option<DeliveryState>,
//         recver: &Option<DeliveryState>,
//     ) -> Expecting {
//         match (sender, recver) {
//             (_, Some(DeliveryState::Declared(_)))
//             | (_, Some(DeliveryState::TransactionalState(_)))
//             | (Some(DeliveryState::Declared(_)), _)
//             | (Some(DeliveryState::TransactionalState(_)), _) => {
//                 // Illegal delivery states?
//                 Expecting::Abort
//             }

//             // delivery-tag 1 example
//             (None, None) => !incomplete_unsettled, // A resend is not expected if incomplete_unsettled is true

//             // delivery-tag 2 and 4 example
//             //
//             // delivery-tag 4 has a null in the remote value, which is equivalent to (0,0) Unlike the
//             // case with delivery-tag 1 the resent delivery MUST be sent with the resume flag set to
//             // true and the delivery-tag set to 4
//             (
//                 None,
//                 Some(DeliveryState::Received(Received {
//                     section_number,
//                     section_offset,
//                 })),
//             ) => true,

//             // delivery-tag 3 example
//             (None, Some(DeliveryState::Accepted(_)))
//             | (None, Some(DeliveryState::Modified(_)))
//             | (None, Some(DeliveryState::Rejected(_)))
//             | (None, Some(DeliveryState::Released(_))) => {
//                 let mut lock = self.unsettled.write().await;
//                 lock.as_mut().and_then(|map| map.remove(&delivery_tag));
//                 false
//             }

//             // delivery-tag 5 example
//             (Some(DeliveryState::Received(_)), None) => {
//                 !incomplete_unsettled
//             }

//             // delivery-tag 6, 7 and 9 examples
//             (
//                 Some(DeliveryState::Received(local_recved)),
//                 Some(DeliveryState::Received(remote_recved)),
//             ) => {
//                 true
//             }

//             // delivery-tag 8 example
//             //
//             // This is just like case 3
//             (Some(DeliveryState::Received(_)), Some(DeliveryState::Accepted(_)))
//             | (Some(DeliveryState::Received(_)), Some(DeliveryState::Modified(_)))
//             | (Some(DeliveryState::Received(_)), Some(DeliveryState::Rejected(_)))
//             | (Some(DeliveryState::Received(_)), Some(DeliveryState::Released(_))) => {
//                 // This will fail if the oneshot receiver is already dropped
//                 // which means the application probably doesn't care about the
//                 // delivery state anyway
//                 let mut lock = self.unsettled.write().await;
//                 lock.as_mut().and_then(|map| map.remove(&delivery_tag));
//                 false
//             }

//             // delivery-tag 10 example
//             //
//             // For delivery-tag 10 the receiver has no record of the delivery. However, in contrast to
//             // the cases of delivery-tag 1 and delivery-tag 5, since it is known that the sender can
//             // only have arrived at this state through knowing that the receiver has received the whole
//             // message (or that the sender had spontaneously reached a terminal outcome with no
//             // possibility of resumption) there is no need to resend the message
//             (Some(DeliveryState::Accepted(_)), None)
//             | (Some(DeliveryState::Modified(_)), None)
//             | (Some(DeliveryState::Rejected(_)), None)
//             | (Some(DeliveryState::Released(_)), None) => {
//                 false
//             }

//             // delivery-tag 11 and 14 case
//             //
//             // For delivery-tag 11 it MUST be assumed that the sender spontaneously attained the
//             // terminal outcome (and is unable to resume). In this case the sender can simply abort the
//             // delivery as it cannot be resumed.
//             //
//             // For delivery-tag 14 the case is essentially the same as for delivery-tag 11, as the null
//             // state at the receiver is essentially identical to having the state Received with
//             // section-number=0 and section-offset=0.
//             (Some(DeliveryState::Accepted(_)), Some(DeliveryState::Received(_)))
//             | (Some(DeliveryState::Modified(_)), Some(DeliveryState::Received(_)))
//             | (Some(DeliveryState::Rejected(_)), Some(DeliveryState::Received(_)))
//             | (Some(DeliveryState::Released(_)), Some(DeliveryState::Received(_))) => {
//                 true
//             }

//             // delivery-tag 12 case
//             //
//             // For delivery-tag 12 both the sender and receiver have attained the same view of the
//             // terminal outcome, but neither has settled. In this case the sender SHOULD simply settle
//             // the delivery.
//             (Some(DeliveryState::Accepted(_)), Some(DeliveryState::Accepted(_)))
//             | (Some(DeliveryState::Modified(_)), Some(DeliveryState::Modified(_)))
//             | (Some(DeliveryState::Rejected(_)), Some(DeliveryState::Rejected(_)))
//             | (Some(DeliveryState::Released(_)), Some(DeliveryState::Released(_))) => {
//                 let mut lock = self.unsettled.write().await;
//                 lock.as_mut().and_then(|map| map.remove(&delivery_tag));
//                 false
//             }

//             // delivery-tag 13 example
//             //
//             // For delivery-tag 13 the sender and receiver have both attained terminal outcomes, but the
//             // outcomes differ. In this case, since the outcome actually takes effect at the sender, it
//             // is the sender’s view that is definitive. The sender thus MUST restate this as the
//             // terminal outcome, and the receiver SHOULD then echo this and settle.
//             (Some(local_state), Some(_)) => true,
//         }
//     }
// }

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
