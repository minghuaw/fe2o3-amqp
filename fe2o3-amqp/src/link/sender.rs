use std::time::Duration;

use fe2o3_amqp_types::{messaging::Message, performatives::Disposition};

use crate::{error::EngineError, session::SessionHandle};

pub struct Sender {
    
}

impl Sender {
    pub async fn attach(session: &mut SessionHandle) -> Result<Self, EngineError> {
        todo!()
    }

    pub async fn send(&mut self, message: Message) -> Result<Disposition, EngineError> {
        todo!()
    }

    pub async fn send_with_timeout(&mut self, message: Message, timeout: Duration) -> Result<Disposition, EngineError> {
        todo!()
    } 

    pub async fn detach(&mut self) -> Result<(), EngineError> {
        todo!()
    }
}

/// TODO: impl `futures_util::io::IntoSink`
pub struct SenderSink {

}

