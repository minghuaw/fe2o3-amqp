use fe2o3_amqp_types::messaging::Message;

use crate::{error::EngineError, session::SessionHandle};

pub mod builder;

pub struct ReceiverLink {
    
}

impl ReceiverLink {
    pub fn builder() -> builder::Builder {
        todo!()
    }

    pub async fn attach(session: &mut SessionHandle, name: impl Into<String>) -> Result<Self, EngineError> {
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
pub struct ReceiverLinkStream {

}