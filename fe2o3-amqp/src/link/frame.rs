use fe2o3_amqp_types::{
    performatives::{Attach, Detach, Disposition, Transfer},
    transaction::TransactionId,
};

use crate::{
    endpoint::{InputHandle, LinkFlow},
    Payload,
};

pub(crate) type LinkIncomingItem = LinkFrame;

/// Link frames.
///
/// This is a subset of the AMPQ frames
#[derive(Debug)]
pub(crate) enum LinkFrame {
    Attach(Attach),
    Flow(LinkFlow),
    Transfer {
        input_handle: InputHandle,
        performative: Transfer,
        payload: Payload,
    },
    Disposition(Disposition),
    Detach(Detach),

    /// Indicating to the receiver that Txn controller side is requesting for
    /// a transactional acquisition
    Acquisition(TransactionId),
}

/// Regular Attach for non-transactional links
#[derive(Debug)]
pub struct RegAttach {}

/// Attach frame for control links
#[derive(Debug)]
pub struct TxnAttach {}
