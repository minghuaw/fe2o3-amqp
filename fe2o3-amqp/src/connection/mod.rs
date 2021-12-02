use std::{cmp::min, collections::BTreeMap, convert::TryInto};

use async_trait::async_trait;

use fe2o3_amqp_types::{
    definitions,
    performatives::{Begin, ChannelMax, Close, End, MaxFrameSize, Open},
};
use futures_util::{Sink, SinkExt};
use slab::Slab;
use tokio::{
    sync::{mpsc::Sender, oneshot},
    task::JoinHandle,
};
use url::Url;

use crate::{
    control::ConnectionControl,
    endpoint,
    error::EngineError,
    session::SessionFrame,
    session::{Session, SessionFrameBody, SessionIncomingItem},
    transport::amqp::{Frame, FrameBody},
};

use self::{builder::WithoutContainerId, engine::SessionId};

pub mod builder;
pub mod engine;
mod error;
pub mod heartbeat;
pub use error::Error;

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
    pub(crate) control: Sender<ConnectionControl>,
    handle: JoinHandle<Result<(), EngineError>>,

    // outgoing channel for session
    pub(crate) outgoing: Sender<SessionFrame>,
    // session_control: Sender<SessionControl>,
}

impl ConnectionHandle {
    pub async fn close(&mut self) -> Result<(), EngineError> {
        self.control.send(ConnectionControl::Close(None)).await?;
        match (&mut self.handle).await {
            Ok(res) => res,
            Err(_) => Err(EngineError::Message("JoinError")),
        }
    }

    pub(crate) async fn create_session(
        &mut self,
        tx: Sender<SessionIncomingItem>,
    ) -> Result<(u16, SessionId), EngineError> {
        let (responder, resp_rx) = oneshot::channel();
        self.control
            .send(ConnectionControl::CreateSession { tx, responder })
            .await?;
        let result = resp_rx
            .await
            .map_err(|_| EngineError::Message("Oneshot sender is dropped"))?;
        result
    }

    // pub(crate) async fn drop_session(&mut self, session_id: SessionId) -> Result<(), EngineError> {
    //     self.control.send(ConnectionControl::DropSession(session_id)).await?;
    //     Ok(())
    // }
}

pub struct Connection {
    control: Sender<ConnectionControl>,

    // local
    local_state: ConnectionState,
    local_open: Open,
    local_sessions: Slab<Sender<SessionIncomingItem>>,
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
        control: Sender<ConnectionControl>,
        local_state: ConnectionState,
        local_open: Open,
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

    fn create_session(
        &mut self,
        tx: Sender<SessionIncomingItem>,
    ) -> Result<(u16, usize), Self::Error> {
        match &self.local_state {
            ConnectionState::Start
            | ConnectionState::HeaderSent
            | ConnectionState::HeaderReceived
            | ConnectionState::HeaderExchange
            | ConnectionState::CloseSent
            | ConnectionState::Discarding
            | ConnectionState::End => return Err(EngineError::Message("Illegal local state")),
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
            entry.insert(tx);
            let channel = session_id as u16; // TODO: a different way of allocating session id?
            self.session_by_outgoing_channel.insert(channel, session_id);
            Ok((channel, session_id))
        }
    }

    fn drop_session(&mut self, session_id: usize) {
        self.local_sessions.remove(session_id);
    }

    /// Reacting to remote Open frame
    async fn on_incoming_open(&mut self, _channel: u16, open: Open) -> Result<(), Self::Error> {
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
    async fn on_incoming_begin(&mut self, channel: u16, begin: Begin) -> Result<(), Self::Error> {
        println!(">>> Debug: on_incoming_begin");
        match &self.local_state {
            ConnectionState::Opened => {}
            // TODO: what about pipelined
            _ => return Err(EngineError::illegal_state()),
        }

        match begin.remote_channel {
            Some(outgoing_channel) => {
                let session_id = self
                    .session_by_outgoing_channel
                    .get(&outgoing_channel)
                    .ok_or_else(|| EngineError::not_found())?;

                if self.session_by_incoming_channel.contains_key(&channel) {
                    return Err(EngineError::not_allowed()); // TODO: this is probably not how not allowed should be used?
                }
                self.session_by_incoming_channel
                    .insert(channel, *session_id);

                // forward begin to session
                let tx = self
                    .local_sessions
                    .get_mut(*session_id)
                    .ok_or_else(|| EngineError::not_found())?;
                let sframe = SessionFrame::new(channel, SessionFrameBody::Begin(begin));
                tx.send(Ok(sframe)).await?;
            }
            None => {
                todo!()
            }
        }

        Ok(())
    }

    /// Reacting to remote End frame
    async fn on_incoming_end(&mut self, channel: u16, end: End) -> Result<(), Self::Error> {
        println!(">>> Debug: on_incoming_end");

        match &self.local_state {
            ConnectionState::Opened => {}
            _ => return Err(EngineError::illegal_state()),
        }

        // Forward to session
        let sframe = SessionFrame::new(channel, SessionFrameBody::End(end));
        // Drop incoming channel
        let session_id = self
            .session_by_incoming_channel
            .remove(&channel)
            .ok_or_else(|| EngineError::not_found())?;
        self.local_sessions
            .get_mut(session_id)
            .ok_or_else(|| EngineError::not_found())?
            .send(Ok(sframe))
            .await?;

        Ok(())
    }

    /// Reacting to remote Close frame
    async fn on_incoming_close(&mut self, _channel: u16, close: Close) -> Result<(), Self::Error> {
        println!(">>> Debug: on_incoming_close");
        match &self.local_state {
            ConnectionState::Opened => {
                self.local_state = ConnectionState::CloseReceived;
                self.control.send(ConnectionControl::Close(None)).await?;
            }
            ConnectionState::CloseSent => self.local_state = ConnectionState::End,
            _ => return Err(EngineError::illegal_state()),
        };

        if let Some(err) = close.error {
            println!("Remote error {:?}", err);
            todo!()
        }
        Ok(())
    }

    async fn send_open<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<Frame> + Send + Unpin,
        W::Error: Into<EngineError>,
    {
        println!(">>> Debug: on_outgoing_open");

        let body = FrameBody::Open(self.local_open.clone());
        let frame = Frame::new(0u16, body);
        writer.send(frame).await.map_err(Into::into)?;

        // change local state after successfully sending the frame
        match &self.local_state {
            ConnectionState::HeaderExchange => self.local_state = ConnectionState::OpenSent,
            ConnectionState::OpenReceived => self.local_state = ConnectionState::Opened,
            ConnectionState::HeaderSent => self.local_state = ConnectionState::OpenPipe,
            s @ _ => return Err(EngineError::UnexpectedConnectionState(s.clone())),
        }

        Ok(())
    }

    fn on_outgoing_begin(&mut self, channel: u16, begin: Begin) -> Result<Frame, Self::Error> {
        println!(">>> Debug: on_outgoing_begin");

        // TODO: the engine already checks that
        // match &self.local_state {
        //     ConnectionState::Opened => {}
        //     _ => return Err(EngineError::Message("Illegal local connection state")),
        // }

        let frame = Frame::new(channel, FrameBody::Begin(begin));
        Ok(frame)
    }

    fn on_outgoing_end(&mut self, channel: u16, end: End) -> Result<Frame, Self::Error> {
        println!(">>> Debug: on_outgoing_end");

        self.session_by_outgoing_channel
            .remove(&channel)
            .ok_or_else(|| EngineError::Message("Local session id is not found"))?;
        let frame = Frame::new(channel, FrameBody::End(end));
        Ok(frame)
    }

    // TODO: set a timeout for recving incoming Close
    async fn send_close<W>(
        &mut self,
        writer: &mut W,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<Frame> + Send + Unpin,
        W::Error: Into<EngineError>,
    {
        let frame = Frame::new(0u16, FrameBody::Close(Close { error }));
        writer.send(frame).await.map_err(Into::into)?;

        match &self.local_state {
            ConnectionState::Opened => self.local_state = ConnectionState::CloseSent,
            ConnectionState::CloseReceived => self.local_state = ConnectionState::End,
            ConnectionState::OpenSent => self.local_state = ConnectionState::ClosePipe,
            ConnectionState::OpenPipe => self.local_state = ConnectionState::OpenClosePipe,
            s @ _ => return Err(EngineError::UnexpectedConnectionState(s.clone())),
        }
        Ok(())
    }

    fn session_tx_by_incoming_channel(
        &mut self,
        channel: u16,
    ) -> Option<&mut Sender<SessionIncomingItem>> {
        let session_id = self.session_by_incoming_channel.get(&channel)?;
        self.local_sessions.get_mut(*session_id)
    }

    fn session_tx_by_outgoing_channel(
        &mut self,
        channel: u16,
    ) -> Option<&mut Sender<SessionIncomingItem>> {
        let session_id = self.session_by_outgoing_channel.get(&channel)?;
        self.local_sessions.get_mut(*session_id)
    }
}
