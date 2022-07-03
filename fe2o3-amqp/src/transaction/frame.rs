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
