use fe2o3_amqp_types::messaging::Message;

use crate::{session::SessionHandle};

use super::{
    builder::{self, WithoutName, WithoutTarget},
    role, Error
};

pub struct Receiver {}

impl Receiver {
    pub fn builder() -> builder::Builder<role::Receiver, WithoutName, WithoutTarget> {
        todo!()
    }

    pub async fn attach(
        session: &mut SessionHandle,
        name: impl Into<String>,
    ) -> Result<Self, Error> {
        todo!()
    }

    pub async fn recv(&mut self) -> Result<Message, Error> {
        todo!()
    }

    pub async fn recv_with_timeout(&mut self) -> Result<Message, Error> {
        todo!()
    }

    pub async fn detach(&mut self) -> Result<(), Error> {
        todo!()
    }
}

/// TODO: impl `futures_util::future::IntoStream`
pub struct ReceiverStream {}
