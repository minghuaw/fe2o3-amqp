use fe2o3_amqp_types::performatives::{Disposition, Transfer};
use tokio::sync::mpsc;

use crate::{endpoint::InputHandle, Payload};

use super::TxnCoordinator;

pub(crate) enum TxnWorkFrame {
    Post{
        input_handle: InputHandle,
        transfer: Transfer,
        payload: Payload,
    },
    Retire(Disposition),
    Acquire()
}

/// Represents the transactional resource
#[derive(Debug)]
pub struct ResourceManager {
    coordinator: TxnCoordinator,
    work_frames: mpsc::Receiver<TxnWorkFrame>,
}

