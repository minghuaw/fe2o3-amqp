use async_trait::async_trait;
use bytes::BytesMut;
use fe2o3_amqp_types::{definitions::{Error, Fields, Handle, SequenceNo, TransferNumber}, performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer}, primitives::Symbol};
use tokio::{sync::mpsc::Sender, task::JoinHandle};

use crate::{control::SessionControl, endpoint, error::EngineError, transport::{amqp::Frame}};

mod frame;
pub use frame::*;
pub mod engine;
pub mod builder;

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
    control: Sender<SessionControl>,
    handle: JoinHandle<Result<(), EngineError>>,

}

pub struct Session {
    control: Sender<SessionControl>,
    session_id: usize,
    outgoing_channel: u16,

    // local amqp states
    local_state: SessionState,    
    next_outgoing_id: TransferNumber,
    incoming_window: TransferNumber,
    outgoing_window: TransferNumber,
    handle_max: Handle,

    // initialize with 0 first and change after receiving the remote Begin
    next_incoming_id: TransferNumber,
    remote_incoming_window: SequenceNo,
    remote_outgoing_window: SequenceNo,

    // capabilities
    offered_capabilities: Option<Vec<Symbol>>,
    desired_capabilities: Option<Vec<Symbol>>,
    properties: Option<Fields>,
}

impl Session {
    pub fn new(
        control: Sender<SessionControl>,
        session_id: usize,
        outgoing_channel: u16,

        // local amqp states
        local_state: SessionState,    
        next_outgoing_id: TransferNumber,
        incoming_window: TransferNumber,
        outgoing_window: TransferNumber,
        handle_max: Handle,

        // initialize with 0 first and change after receiving the remote Begin
        next_incoming_id: TransferNumber,
        remote_incoming_window: SequenceNo,
        remote_outgoing_window: SequenceNo,

        // capabilities
        offered_capabilities: Option<Vec<Symbol>>,
        desired_capabilities: Option<Vec<Symbol>>,
        properties: Option<Fields>,
    ) -> Self {
        Self {
            control,
            session_id,
            outgoing_channel,
            local_state,
            next_outgoing_id,
            incoming_window,
            outgoing_window,
            handle_max,
            next_incoming_id,
            remote_incoming_window,
            remote_outgoing_window,
            offered_capabilities,
            desired_capabilities,
            properties
        }
    }
}

#[async_trait]
impl endpoint::Session for Session {
    type Error = EngineError;
    type State = SessionState;

    fn local_state(&self) -> &Self::State {
        &self.local_state
    }

    fn local_state_mut(&mut self) -> &mut Self::State {
        &mut self.local_state
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