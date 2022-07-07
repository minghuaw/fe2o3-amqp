use fe2o3_amqp_types::performatives::{Attach, Detach, Disposition, Transfer};

use crate::{
    endpoint::{InputHandle, LinkFlow},
    Payload,
};

#[cfg(feature = "transaction")]
use fe2o3_amqp_types::transaction::TransactionId;

pub(crate) type LinkIncomingItem = LinkFrame;

/// Link frames.
///
/// This is a subset of the AMPQ frames
// #[derive(Debug)]
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

    #[cfg(feature = "transaction")]
    /// Indicating to the receiver that Txn controller side is requesting for
    /// a transactional acquisition
    Acquisition(TransactionId),
}

impl std::fmt::Debug for LinkFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attach(arg0) => f.debug_tuple("Attach").field(arg0).finish(),
            Self::Flow(arg0) => f.debug_tuple("Flow").field(arg0).finish(),
            Self::Transfer {
                input_handle,
                performative,
                payload,
            } => f
                .debug_struct("Transfer")
                .field("input_handle", input_handle)
                .field("performative", performative)
                .field("payload.len", &payload.len())
                .finish(),
            Self::Disposition(arg0) => f.debug_tuple("Disposition").field(arg0).finish(),
            Self::Detach(arg0) => f.debug_tuple("Detach").field(arg0).finish(),
            #[cfg(feature = "transaction")]
            Self::Acquisition(arg0) => f.debug_tuple("Acquisition").field(arg0).finish(),
        }
    }
}
