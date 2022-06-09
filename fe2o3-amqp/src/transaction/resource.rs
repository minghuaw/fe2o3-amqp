use fe2o3_amqp_types::performatives::{Disposition, Transfer};
use tokio::sync::mpsc;

use crate::{endpoint::InputHandle, Payload, transaction::control_link_frame::ControlMessageBody};

use super::{TxnCoordinator, TxnWorkFrame};

/// Represents the transactional resource
#[derive(Debug)]
pub struct ResourceManager {
    coordinator: TxnCoordinator,
    work_frames: mpsc::Receiver<TxnWorkFrame>,
}

impl ResourceManager {
    async fn event_loop(mut self) {
        loop {
            tokio::select! {
                delivery = self.coordinator.inner.recv_inner::<ControlMessageBody>() => {

                }
            }
        }
    }
}
