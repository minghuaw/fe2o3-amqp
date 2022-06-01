use fe2o3_amqp_types::performatives::{Disposition, Transfer};
use tokio::sync::mpsc;

use crate::{endpoint::InputHandle, Payload};

use super::TxnCoordinator;

// /// Represents the transactional resource
// #[derive(Debug)]
// pub struct ResourceManager {
//     coordinator: TxnCoordinator,
//     work_frames: mpsc::Receiver<TxnWorkFrame>,
// }

