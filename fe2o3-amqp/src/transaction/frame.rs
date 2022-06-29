//! Transactional work frames

use fe2o3_amqp_types::performatives::{Disposition, Transfer};

use crate::Payload;

/// Transactional work
#[derive(Debug)]
pub(crate) enum TxnWorkFrame {
    Post {
        transfer: Transfer,
        payload: Payload,
    },
    Retire(Disposition),
    // Acquire(Flow), // Not implemented for now
}

// impl TxnWorkFrame {
//     pub fn txn_id(&self) -> Option<&TransactionId> {
//         match self {
//             TxnWorkFrame::Post { transfer, .. } => match &transfer.state {
//                 Some(DeliveryState::TransactionalState(state)) => Some(&state.txn_id),
//                 _ => None,
//             },
//             TxnWorkFrame::Retire(disposition) => match &disposition.state {
//                 Some(DeliveryState::TransactionalState(state)) => Some(&state.txn_id),
//                 _ => None,
//             },
//             TxnWorkFrame::Acquire(flow) => {
//                 let key = Symbol::from(TXN_ID_KEY);
//                 match flow.properties.as_ref().map(|m| m.get(&key)) {
//                     Some(Some(Value::Binary(value))) => Some(value),
//                     _ => None,
//                 }
//             },
//         }
//     }
// }
