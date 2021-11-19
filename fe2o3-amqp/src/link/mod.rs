mod frame;
use fe2o3_amqp_types::{definitions::{Fields, SequenceNo}, performatives::Flow};
pub use frame::*;
pub mod builder;
pub mod receiver;
pub mod receiver_link;
pub mod sender;
pub mod sender_link;

pub use receiver::Receiver;
pub use sender::Sender;

use crate::error::EngineError;

pub mod role {

    /// Type state for link::builder::Builder
    pub struct Sender {}

    /// Type state for link::builder::Builder
    pub struct Receiver {}
}

pub enum LinkState {
    /// The initial state after initialization
    Unattached,

    /// An attach frame has been sent
    AttachSent,

    /// An attach frame has been received
    AttachReceived,

    /// The link is attached
    Attached,

    /// A detach frame has been sent
    DetachSent,

    /// A detach frame has been received
    DetachReceived,

    /// The link is detached
    Detached,
}

pub(crate) struct LinkFlowState {
    delivery_count: SequenceNo,
    link_credit: u32,
    avaiable: u32,
    drain: bool,
    properties: Option<Fields>,
}

impl LinkFlowState {
    pub(crate) fn on_incoming_flow(&mut self, flow: &Flow) -> Result<Option<Flow>, EngineError> {
        todo!()
    }
}