use std::sync::{Arc, atomic::AtomicU64};

use fe2o3_amqp_types::{
    messaging::{DeliveryState, Body},
    performatives::{Disposition, Transfer},
    transaction::{Coordinator, Declare, Discharge}, definitions::{self, AmqpError},
};

use crate::{
    endpoint::LinkFlow,
    link::{receiver::ReceiverInner, role, Link, ReceiverFlowState, self}, transaction::control_link_frame::ControlMessageBody, Delivery, util::Running,
};

pub(crate) type CoordinatorLink =
    Link<role::Receiver, Coordinator, ReceiverFlowState, DeliveryState>;

// /// State of transaction coordinator
// pub enum TxnCoordinatorState {
//     /// The control link is established
//     Established,

//     /// 
// }

// pub(crate) mod coordinator_state {
//     pub struct Established {}

//     pub struct TxnDeclared {}

//     pub struct Closed {}
// }

/// Transaction coordinator
#[derive(Debug)]
pub struct TxnCoordinator {
    pub(crate) inner: ReceiverInner<CoordinatorLink>,
}

impl TxnCoordinator {
    fn on_declare(&mut self, declare: &Declare) {
        todo!()
    }

    fn on_discharge(&mut self, discharge: &Discharge) {
        todo!()
    }

    async fn on_delivery(&mut self, delivery: Delivery<ControlMessageBody>) -> Result<Running, link::Error> {
        if let Body::Value(msg) = delivery.body() {
            match &msg.0 {
                ControlMessageBody::Declare(declare) => todo!(),
                ControlMessageBody::Discharge(discharge) => todo!(),
            }
        } else {
            let error = definitions::Error::new(
                AmqpError::NotAllowed,
                "Transaction coordinator expected Declare or Discharge message only".to_string(),
                None
            );
            
            Err(link::Error::Local(error))
        }
    }

    async fn handle_error(&mut self, error: link::Error) -> Running {
        match error {
            link::Error::Local(error) => {
                let _ = self.inner.close_with_error(Some(error)).await;
                Running::Stop
            },
            link::Error::Detached(_) => todo!(),
            link::Error::Rejected(_) => todo!(),
            link::Error::Released(_) => todo!(),
            link::Error::Modified(_) => todo!(),
        }
    }

    async fn event_loop(mut self) {
        loop {
            let result = tokio::select! {
                result = self.inner.recv_inner::<ControlMessageBody>() => match result {
                    Ok(Some(delivery)) => self.on_delivery(delivery).await,
                    Ok(None) => Ok(Running::Continue), // incomplete transfer, wait for more transfers
                    Err(error) => Err(error),
                }
            };

            let running = match result {
                Ok(running) => running ,
                Err(error) => self.handle_error(error).await,
            };

            if let Running::Stop = running {
                break
            }
        }
    }
}
