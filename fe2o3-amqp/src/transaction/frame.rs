//! Transactional work frames

use fe2o3_amqp_types::performatives::{Transfer, Disposition, Flow};

use crate::{endpoint::InputHandle, Payload};

/// Transactional work 
#[derive(Debug)]
pub(crate) enum TransactionalWork {
    Post {
        input_handle: InputHandle,
        transfer: Transfer,
        payload: Payload,
    },
    Retire(Disposition),
    Acquire(Flow),
}