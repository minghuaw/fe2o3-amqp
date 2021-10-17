use std::cmp::min;

use async_trait::async_trait;

use fe2o3_amqp_types::{definitions::{Fields, IetfLanguageTag, Milliseconds}, performatives::{Begin, ChannelMax, Close, End, MaxFrameSize, Open}, primitives::Symbol};
use futures_util::{Sink, SinkExt};
use tokio::{sync::mpsc::{UnboundedSender}, task::JoinHandle};

use crate::{control::{ConnectionControl, SessionControl}, endpoint, error::EngineError, session::Session, transport::{amqp::{Frame, FrameBody}, connection::ConnectionState}};

use self::builder::WithoutContainerId;

pub mod builder;

pub struct ConnectionHandle {
    control: UnboundedSender<ConnectionControl>,
    handle: JoinHandle<Result<(), EngineError>>,

    session_control: UnboundedSender<SessionControl>,
}

pub struct Connection {
    // local 
    local_state: ConnectionState,
    local_open: Open,

    // remote 
    remote_open: Option<Open>,

    // mutually agreed channel max
    agreed_channel_max: u16,
}

/* ------------------------------- Public API ------------------------------- */
impl Connection {
    pub fn builder() -> builder::Builder<WithoutContainerId> {
        builder::Builder::new()
    }
}

/* ------------------------------- Private API ------------------------------ */
impl Connection {
    fn new(
        local_state: ConnectionState, 
        local_open: Open
    ) -> Self {
        let agreed_channel_max = local_open.channel_max.0;
        Self {
            local_state,
            local_open,
            remote_open: None,
            agreed_channel_max
        }
    }
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

    fn local_open(&self) -> &Open {
        &self.local_open
    }

    /// Reacting to remote Open frame
    async fn on_incoming_open(&mut self, channel: u16, open: Open) -> Result<(), Self::Error> {
        match &self.local_state {
            ConnectionState::HeaderExchange => self.local_state = ConnectionState::OpenReceived,
            ConnectionState::OpenSent => self.local_state = ConnectionState::Opened,
            ConnectionState::ClosePipe => self.local_state = ConnectionState::CloseSent,
            _ => return Err(EngineError::illegal_state()),
        }

        // set channel_max to mutually acceptable
        self.agreed_channel_max = min(self.local_open.channel_max.0, open.channel_max.0);
        self.remote_open = Some(open);
        
        Ok(())
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

    async fn on_outgoing_open<W>(&mut self, writer: &mut W, channel: u16, open: Open) -> Result<(), Self::Error> 
    where 
        W: Sink<Frame, Error = EngineError> + Send + Unpin,
    {
        println!(">>> Debug: on_outgoing_open");

        let body = FrameBody::open(open.clone());
        let frame = Frame::new(channel, body);
        writer.send(frame).await?;

        // change local state after successfully sending the frame
        match &self.local_state {
            ConnectionState::HeaderExchange => self.local_state = ConnectionState::OpenSent,
            ConnectionState::OpenReceived => self.local_state = ConnectionState::Opened,
            ConnectionState::HeaderSent => self.local_state = ConnectionState::OpenPipe,
            s @ _ => return Err(EngineError::UnexpectedConnectionState(s.clone())),
        }
        
        Ok(())
    }

    async fn on_outgoing_begin<W>(&mut self, writer: &mut W, channel: u16, begin: Begin) -> Result<(), Self::Error>
        where W: Sink<Frame, Error = EngineError> + Send + Unpin
    {
        todo!()
    }

    async fn on_outgoing_end<W>(&mut self, writer: &mut W, channel: u16, end: End) -> Result<(), Self::Error>
        where W: Sink<Frame, Error = EngineError> + Send + Unpin
    {
        todo!()
    }

    async fn on_outgoing_close<W>(&mut self, writer: &mut W, channel: u16, close: Close) -> Result<(), Self::Error>
        where W: Sink<Frame, Error = EngineError> + Send + Unpin 
    {
        todo!()
    }

    fn session_mut_by_incoming_channel(&mut self, channel: u16) -> Result<&mut Self::Session, Self::Error> {
        todo!()
    }

    fn session_mut_by_outgoing_channel(&mut self, channel: u16) -> Result<&mut Self::Session, Self::Error> {
        todo!()
    }
}

