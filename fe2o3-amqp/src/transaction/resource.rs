use fe2o3_amqp_types::performatives::{Disposition, Transfer};
use tokio::sync::mpsc;

use crate::{endpoint::InputHandle, Payload, transaction::control_link_frame::ControlMessageBody};

use super::{TxnCoordinator, TxnWorkFrame};

/// Represents the transactional resource
#[derive(Debug, Clone)]
pub struct ResourceTransaction {
    buf: Vec<TxnWorkFrame>
}

impl ResourceTransaction {

}
