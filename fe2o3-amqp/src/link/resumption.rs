use fe2o3_amqp_types::messaging::DeliveryState;

use crate::Payload;

use super::delivery::UnsettledMessage;

pub(crate) struct ResumingDelivery {
    resume: bool,
    state: Option<DeliveryState>,
    payload: Payload,
}

fn should_resume_delivery(local: &UnsettledMessage, remote: &Option<DeliveryState>) -> Option<ResumingDelivery> {
    match (local.state(), remote) {
        // Illegal delivery states?
        (_, Some(DeliveryState::Declared(_)))
        | (_, Some(DeliveryState::TransactionalState(_))) 
        | (Some(DeliveryState::Declared(_)), _)
        | (Some(DeliveryState::TransactionalState(_)), _) => todo!(), // TODO: IllegalDeliveryState?

        // delivery-tag 1 example
        (None, None) => {
            Some(ResumingDelivery {
                resume: false,
                state: None,
                payload: local.payload().clone() // cloning `Bytes` is cheap
            })
        },

        // delivery-tag 2 example
        (None, Some(DeliveryState::Received(_))) => todo!(),
        
        // delivery-tag 3 example
        (None, Some(DeliveryState::Accepted(_))) 
        | (None, Some(DeliveryState::Modified(_))) 
        | (None, Some(DeliveryState::Rejected(_))) 
        | (None, Some(DeliveryState::Released(_))) => todo!(),
        
        // delivery-tag 5 example
        (Some(DeliveryState::Received(_)), None) => todo!(),

        // delivery-tag 10 example
        (Some(DeliveryState::Accepted(_)), None) 
        | (Some(DeliveryState::Modified(_)), None) 
        | (Some(DeliveryState::Rejected(_)), None) 
        | (Some(DeliveryState::Released(_)), None) => todo!(),

        // delivery-tag 6 and 7 examples
        (Some(DeliveryState::Received(_)), Some(DeliveryState::Received(_))) => todo!(),

        // delivery-tag 11 example
        (Some(DeliveryState::Accepted(_)), Some(DeliveryState::Received(_))) 
        | (Some(DeliveryState::Modified(_)), Some(DeliveryState::Received(_))) 
        | (Some(DeliveryState::Rejected(_)), Some(DeliveryState::Received(_))) 
        | (Some(DeliveryState::Released(_)), Some(DeliveryState::Received(_))) => todo!(),

        // delivery-tag 8 example
        (Some(DeliveryState::Received(_)), Some(DeliveryState::Accepted(_))) 
        | (Some(DeliveryState::Received(_)), Some(DeliveryState::Modified(_))) 
        | (Some(DeliveryState::Received(_)), Some(DeliveryState::Rejected(_))) 
        | (Some(DeliveryState::Received(_)), Some(DeliveryState::Released(_))) => todo!(),
        
     
        // local_state is terminal, remote state is terminal
        // delivery-tag 13 example
        (Some(local_state), Some(remote_state)) => todo!(),
    }
}