use async_trait::async_trait;
use bytes::BytesMut;
use fe2o3_amqp_types::{definitions::Error, performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer}};
use tokio::sync::mpsc::Sender;

use crate::{endpoint, error::EngineError, transport::{amqp::Frame}};

mod frame;
pub use frame::*;
pub mod engine;

// 2.5.5 Session States
// UNMAPPED
// BEGIN SENT
// BEGIN RCVD
// MAPPED END SENT
// END RCVD
// DISCARDING
pub enum SessionState {
    Unmapped,

    BeginSent,

    BeginReceived,

    Mapped,
    
    EndSent,

    EndReceived,

    Discarding,
}


pub struct SessionHandle { 

}

pub struct Session {

}

#[async_trait]
impl endpoint::Session for Session {
    type Error = EngineError;
    type State = SessionState;

    fn local_state(&self) -> &Self::State {
        todo!()
    }

    fn local_state_mut(&mut self) -> &mut Self::State {
        todo!()
    }

    async fn on_incoming_begin(&mut self, begin: Begin) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_flow(&mut self, flow: Flow) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_transfer(&mut self, transfer: Transfer, payload: Option<BytesMut>) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_disposition(&mut self, disposition: Disposition) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_end(&mut self, end: End) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_outgoing_begin(&mut self, writer: &mut Sender<SessionFrame>) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error> {
        todo!()
    }

    async fn on_outgoing_flow(&mut self, flow: Flow) -> Result<SessionFrame, Self::Error> {
        todo!()
    }

    async fn on_outgoing_transfer(&mut self, transfer: Transfer, payload: Option<BytesMut>) -> Result<SessionFrame, Self::Error> {
        todo!()
    }

    async fn on_outgoing_disposition(&mut self, disposition: Disposition) -> Result<SessionFrame, Self::Error> {
        todo!()
    }

    async fn on_outgoing_detach(&mut self, detach: Detach) -> Result<SessionFrame, Self::Error> {
        todo!()
    }

    async fn on_outgoing_end(&mut self, writer: &mut Sender<SessionFrame>, error: Option<Error>) -> Result<(), Self::Error> {
        todo!()
    }
}