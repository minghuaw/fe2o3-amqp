use std::{marker::PhantomData, sync::Arc};

use fe2o3_amqp_types::messaging::{Message, Target, Address};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{session::SessionHandle, control::SessionControl};

use super::{
    builder::{self, WithoutName, WithoutTarget, WithTarget},
    role, Error, LinkFrame, state::{LinkState, LinkFlowState}, type_state::{Detached, Attached},
};

pub struct Receiver<S> {
    pub(crate) link: super::Link<role::Receiver, Arc<LinkFlowState>>,
    pub(crate) buffer_size: usize,

    // Control sender to the session
    pub(crate) session: mpsc::Sender<SessionControl>,
    
    // Outgoing mpsc channel to send the Link Frames
    pub(crate) outgoing: PollSender<LinkFrame>,
    pub(crate) incoming: ReceiverStream<LinkFrame>,

    pub(crate) marker: PhantomData<S>,
}

impl Receiver<Detached> {
    pub fn builder() -> builder::Builder<role::Receiver, WithoutName, WithTarget> {
        builder::Builder::new().target(Target::builder().build())
    }

    pub async fn attach(
        session: &mut SessionHandle,
        name: impl Into<String>,
        addr: impl Into<Address>,
    ) -> Result<Receiver<Attached>, Error> {
        Self::builder()
            .name(name)
            .source(addr)
            .attach(session)
            .await
    }
}

//     pub async fn recv(&mut self) -> Result<Message, Error> {
//         todo!()
//     }

//     pub async fn recv_with_timeout(&mut self) -> Result<Message, Error> {
//         todo!()
//     }

//     pub async fn detach(&mut self) -> Result<(), Error> {
//         todo!()
//     }
// }
