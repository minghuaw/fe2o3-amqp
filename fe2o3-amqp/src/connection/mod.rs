use std::{cmp::min, collections::BTreeMap, convert::TryInto};

use async_trait::async_trait;

use fe2o3_amqp_types::{definitions::{Fields, IetfLanguageTag, Milliseconds}, performatives::{Begin, ChannelMax, Close, End, MaxFrameSize, Open}, primitives::Symbol};
use futures_util::{Sink, SinkExt};
use slab::Slab;
use tokio::{sync::mpsc::{self, UnboundedReceiver, UnboundedSender}, task::JoinHandle};
use url::Url;

use crate::{control::{ConnectionControl, SessionControl}, endpoint, error::EngineError, session::Session, transport::{amqp::{Frame, FrameBody}}, session::SessionFrame};

use self::builder::WithoutContainerId;

pub mod builder;
pub mod engine;
pub mod heartbeat;


#[derive(Debug, Clone)]
pub enum ConnectionState {
    Start,

    HeaderReceived,

    HeaderSent,

    HeaderExchange,

    OpenPipe,

    OpenClosePipe,

    OpenReceived,

    OpenSent,

    ClosePipe,

    Opened,

    CloseReceived,

    CloseSent,

    Discarding,

    End,
}

pub struct ConnectionHandle {
    control: UnboundedSender<ConnectionControl>,
    handle: JoinHandle<Result<(), EngineError>>,

    // session_control: UnboundedSender<SessionControl>,
}

impl ConnectionHandle {
    pub async fn close(self) -> Result<(), EngineError> {
        self.control.send(ConnectionControl::Close(None))?;
        match (self.handle).await {
            Ok(res) => res,
            Err(_) => Err(EngineError::Message("JoinError")),
        }
    }
}

pub struct Connection {
    control: UnboundedSender<ConnectionControl>,

    // local 
    local_state: ConnectionState,
    local_open: Open,
    local_sessions: Slab<UnboundedSender<SessionFrame>>,
    session_by_incoming_channel: BTreeMap<u16, usize>,
    session_by_outgoing_channel: BTreeMap<u16, usize>,

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

    pub async fn open(
        container_id: String,
        max_frame_size: impl Into<MaxFrameSize>,
        channel_max: impl Into<ChannelMax>,
        url: impl TryInto<Url, Error = url::ParseError>,
    ) -> Result<ConnectionHandle, EngineError> {
        Connection::builder()
            .container_id(container_id)
            .max_frame_size(max_frame_size)
            .channel_max(channel_max)
            .open(url)
            .await
    }
}

/* ------------------------------- Private API ------------------------------ */
impl Connection {
    fn new(
        control: UnboundedSender<ConnectionControl>,
        local_state: ConnectionState, 
        local_open: Open
    ) -> Self {
        let agreed_channel_max = local_open.channel_max.0;
        Self {
            control,
            local_state,
            local_open,
            local_sessions: Slab::new(),
            session_by_incoming_channel: BTreeMap::new(),
            session_by_outgoing_channel: BTreeMap::new(),

            remote_open: None,
            agreed_channel_max,
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

    async fn create_session(&mut self) -> Result<(u16, UnboundedReceiver<SessionFrame>), Self::Error> {
        match &self.local_state {
            ConnectionState::Start 
            | ConnectionState::HeaderSent
            | ConnectionState::HeaderReceived
            | ConnectionState::HeaderExchange
            | ConnectionState::CloseSent
            | ConnectionState::Discarding
            | ConnectionState::End => {
                return Err(EngineError::Message("Illegal local state"))
            },
            // TODO: what about pipelined open?
            _ => {}
        };

        // get new entry index
        let entry = self.local_sessions.vacant_entry();
        let session_id = entry.key();

        // check if there is enough
        if session_id > self.agreed_channel_max as usize {
            return Err(EngineError::Message(
                "Exceeding max number of channel is not allowed",
            ));
        } else {
            let (tx, rx) = mpsc::unbounded_channel();
            entry.insert(tx);
            let channel = session_id as u16; // TODO: a different way of allocating session id?
            self.session_by_outgoing_channel.insert(channel, session_id);
            Ok((channel, rx))
        }
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
    async fn on_incoming_close(&mut self, _channel: u16, close: Close) -> Result<(), Self::Error> {
        println!(">>> Debug: on_incoming_close");
        match &self.local_state {
            ConnectionState::Opened => {
                self.local_state = ConnectionState::CloseReceived;
                self.control.send(ConnectionControl::Close(None))?;
            },
            ConnectionState::CloseSent => self.local_state = ConnectionState::End,
            _ => return Err(EngineError::illegal_state()),
        };

        if let Some(err) = close.error {
            println!("Remote error {:?}", err);
            todo!()
        }
        Ok(())
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

    // TODO: set a timeout for recving incoming Close
    async fn on_outgoing_close<W>(&mut self, writer: &mut W, channel: u16, close: Close) -> Result<(), Self::Error>
        where W: Sink<Frame, Error = EngineError> + Send + Unpin 
    {
        let frame = Frame::new(
            channel, 
            FrameBody::Close(close),
        );
        writer.send(frame).await?;

        match &self.local_state {
            ConnectionState::Opened => self.local_state = ConnectionState::CloseSent,
            ConnectionState::CloseReceived => self.local_state = ConnectionState::End,
            ConnectionState::OpenSent => self.local_state = ConnectionState::ClosePipe,
            ConnectionState::OpenPipe => self.local_state = ConnectionState::OpenClosePipe,
            s @ _ => return Err(EngineError::UnexpectedConnectionState(s.clone())),
        }
        Ok(())
    }

    fn session_tx_by_incoming_channel(&mut self, channel: u16) -> Option<&mut UnboundedSender<SessionFrame>> {
        let session_id = self.session_by_incoming_channel.get(&channel)?;
        self.local_sessions.get_mut(*session_id)
    }

    fn session_tx_by_outgoing_channel(&mut self, channel: u16) -> Option<&mut UnboundedSender<SessionFrame>> {
        let session_id = self.session_by_outgoing_channel.get(&channel)?;
        self.local_sessions.get_mut(*session_id)
    }
}

