use fe2o3_amqp_types::messaging::Message;

use crate::{error::EngineError, session::SessionHandle};

pub struct Receiver {
    
}

impl Receiver {
    pub async fn attach(session: &mut SessionHandle) -> Result<Self, EngineError> {
        todo!()
    }

    pub async fn recv(&mut self) -> Result<Message, EngineError> {
        todo!()
    }

    pub async fn recv_with_timeout(&mut self) -> Result<Message, EngineError> {
        todo!()
    }

    pub async fn detach(&mut self) -> Result<(), EngineError> {
        todo!()
    }
}

/// TODO: impl `futures_util::future::IntoStream`
pub struct ReceiverStream {

}