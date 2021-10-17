use async_trait::async_trait;

use fe2o3_amqp_types::{definitions::{Fields, IetfLanguageTag, Milliseconds}, performatives::{Begin, ChannelMax, Close, End, MaxFrameSize, Open}, primitives::Symbol};
use tokio::sync::{Semaphore, mpsc::Sender};

use crate::{control::ConnectionControl, endpoint, error::EngineError, session::Session, transport::{amqp::{Frame, FrameBody}, connection::ConnectionState}};

pub struct ConnectionHandle {
    control: Sender<ConnectionControl>,
}

pub struct Connection {
    // local 
    local_state: ConnectionState,
    local_open: Open,

    // remote 
    remote_open: Option<Open>,
}

#[async_trait]
impl endpoint::Connection for Connection {
    type Error = EngineError;
    type State = ConnectionState;
    type Session = Session;

    fn local_state(&self) -> &Self::State {
        &self.local_state
    }

    fn local_state_mut(&mut self) -> &mut Self::State {
        &mut self.local_state
    }

    /// Reacting to remote Open frame
    async fn on_incoming_open(&mut self, channel: u16, open: Open) -> Result<(), Self::Error> {
        todo!()
    }

    /// Reacting to remote Begin frame
    async fn on_incoming_begin(&mut self, channel: u16, begin: &mut Begin) -> Result<(), Self::Error> {
        todo!()
    }

    /// Reacting to remote End frame
    async fn on_incoming_end(&mut self, channel: u16, end: &mut End) -> Result<(), Self::Error> {
        todo!()
    }

    /// Reacting to remote Close frame
    async fn on_incoming_close(&mut self, channel: u16, close: Close) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_outgoing_open(&mut self, channel: u16, open: Open) -> Result<Frame, Self::Error> {
        let body = FrameBody::open(open.clone());
        let frame = Frame::new(channel, body);
        Ok(frame)
    }

    async fn on_outgoing_begin(&mut self, channel: u16, begin: Begin) -> Result<Frame, Self::Error> {
        todo!()
    }

    async fn on_outgoing_end(&mut self, channel: u16, end: End) -> Result<Frame, Self::Error> {
        todo!()
    }

    async fn on_outgoing_close(&mut self, channel: u16, close: Close) -> Result<Frame, Self::Error> {
        todo!()
    }

    fn session_mut_by_incoming_channel(&mut self, channel: u16) -> Result<&mut Self::Session, Self::Error> {
        todo!()
    }

    fn session_mut_by_outgoing_channel(&mut self, channel: u16) -> Result<&mut Self::Session, Self::Error> {
        todo!()
    }
}

