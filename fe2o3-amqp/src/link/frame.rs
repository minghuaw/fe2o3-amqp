use fe2o3_amqp_types::performatives::{Attach, Detach, Disposition, Transfer};

use crate::{
    endpoint::{InputHandle, LinkFlow},
    Payload,
};

#[cfg(feature = "transaction")]
use fe2o3_amqp_types::transaction::TransactionId;

pub(crate) type LinkIncomingItem = LinkFrame;

/// The delivery-independent parts of a transfer frame.
pub(crate) struct TransferFrame {
    pub(crate) input_handle: InputHandle,
    pub(crate) performative: Transfer,
    pub(crate) payload: Payload,
}

/// Link frames.
///
/// This is a subset of the AMPQ frames
// #[derive(Debug)]
pub(crate) enum LinkFrame {
    Attach(Attach),
    Flow(LinkFlow),
    /// A transfer received from the remote peer and forwarded to a receiver
    /// link.
    ///
    /// The receiver derives whether this is the first frame of a delivery or a
    /// continuation from its own incomplete-transfer state. The remote peer
    /// may repeat the delivery-id/tag on continuation frames, so their
    /// presence must not be used for classification.
    RemoteTransfer(TransferFrame),
    /// The first transfer of a locally sent delivery.
    ///
    /// The session assigns the delivery-id to this frame. Continuation frames
    /// of the same delivery use [`LinkFrame::TransferContinuation`], which the
    /// session never assigns a delivery-id to.
    FirstTransfer(TransferFrame),
    /// A continuation of a locally sent delivery whose first frame was
    /// [`LinkFrame::FirstTransfer`].
    TransferContinuation(TransferFrame),
    Disposition(Disposition),
    Detach(Detach),

    #[cfg(feature = "transaction")]
    /// Indicating to the receiver that Txn controller side is requesting for
    /// a transactional acquisition
    Acquisition(TransactionId),
}

/// Clear the fields that may only be set on the first transfer of a delivery.
///
/// Per AMQP 1.0 §2.7.5, continuation transfers may omit the delivery-id,
/// delivery-tag and message-format, and inherit the settled / rcv-settle-mode
/// of the first transfer. Clearing them here keeps the link-level and
/// encoder-level splitters in sync.
pub(crate) fn clear_continuation_fields(transfer: &mut Transfer) {
    transfer.delivery_id = None;
    transfer.delivery_tag = None;
    transfer.message_format = None;
    transfer.settled = None;
    transfer.rcv_settle_mode = None;
}

impl std::fmt::Debug for LinkFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attach(arg0) => f.debug_tuple("Attach").field(arg0).finish(),
            Self::Flow(arg0) => f.debug_tuple("Flow").field(arg0).finish(),
            Self::RemoteTransfer(frame) => fmt_transfer(f, "RemoteTransfer", frame),
            Self::FirstTransfer(frame) => fmt_transfer(f, "FirstTransfer", frame),
            Self::TransferContinuation(frame) => fmt_transfer(f, "TransferContinuation", frame),
            Self::Disposition(arg0) => f.debug_tuple("Disposition").field(arg0).finish(),
            Self::Detach(arg0) => f.debug_tuple("Detach").field(arg0).finish(),
            #[cfg(feature = "transaction")]
            Self::Acquisition(arg0) => f.debug_tuple("Acquisition").field(arg0).finish(),
        }
    }
}

fn fmt_transfer(
    f: &mut std::fmt::Formatter<'_>,
    name: &'static str,
    frame: &TransferFrame,
) -> std::fmt::Result {
    f.debug_struct(name)
        .field("input_handle", &frame.input_handle)
        .field("performative", &frame.performative)
        .field("payload.len", &frame.payload.len())
        .finish()
}
