use std::{marker::PhantomData, sync::Arc};

use fe2o3_amqp_types::messaging::Message;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{session::SessionHandle, control::SessionControl};

use super::{
    builder::{self, WithoutName, WithoutTarget},
    role, Error, LinkFrame, state::LinkState,
};

pub struct Receiver<S> {
    pub(crate) link: super::Link<role::Receiver, Arc<LinkState>>,
    pub(crate) buffer_size: usize,

    // Control sender to the session
    pub(crate) session: mpsc::Sender<SessionControl>,
    
    // Outgoing mpsc channel to send the Link Frames
    pub(crate) outgoing: PollSender<LinkFrame>,
    pub(crate) incoming: ReceiverStream<LinkFrame>,

    pub(crate) marker: PhantomData<S>,
}

// impl Receiver {
//     pub fn builder() -> builder::Builder<role::Receiver, WithoutName, WithoutTarget> {
//         todo!()
//     }

//     pub async fn attach(
//         session: &mut SessionHandle,
//         name: impl Into<String>,
//     ) -> Result<Self, Error> {
//         todo!()
//     }

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
