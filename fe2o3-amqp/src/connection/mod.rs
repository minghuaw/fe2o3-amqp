//! Implementation of AMQP 1.0 Connection

use std::{cmp::min, collections::BTreeMap, convert::TryInto, io};

use async_trait::async_trait;

use fe2o3_amqp_types::{
    definitions::{self, AmqpError},
    performatives::{Begin, ChannelMax, Close, End, MaxFrameSize, Open},
};
use futures_util::{Sink, SinkExt};
use slab::Slab;
use tokio::{
    sync::{mpsc::Sender, oneshot},
    task::JoinHandle,
};
use tracing::{instrument, trace};
use url::Url;

use crate::{
    control::ConnectionControl,
    endpoint,
    frames::amqp::{Frame, FrameBody},
    session::SessionFrame,
    session::{Session, SessionFrameBody, SessionIncomingItem},
};

use self::{builder::WithoutContainerId, engine::SessionId};

pub mod builder;
pub mod engine;
mod error;
pub mod heartbeat;
pub use error::*;

/// Connection states as defined in the AMQP 1.0 Protocol Part 2.4.6
#[derive(Debug, Clone)]
pub enum ConnectionState {
    /// In this state a connection exists, but nothing has been sent or received. This is the state an
    /// implementation would be in immediately after performing a socket connect or socket accept
    Start,

    /// In this state the connection header has been received from the peer but a connection header
    /// has not been sent.
    HeaderReceived,

    /// In this state the connection header has been sent to the peer but no connection header has
    /// been received.
    HeaderSent,

    /// In this state the connection header has been sent to the peer and a connection header has
    /// been received from the peer.
    HeaderExchange,

    /// In this state both the connection header and the open frame have been sent but nothing has
    /// been received.
    OpenPipe,

    /// In this state, the connection header, the open frame, any pipelined connection traffic, and
    /// the close frame have been sent but nothing has been received.
    OpenClosePipe,

    /// In this state the connection headers have been exchanged. An open frame has been received 
    /// from the peer but an open frame has not been sent.
    OpenReceived,

    /// In this state the connection headers have been exchanged. An open frame has been sent
    /// to the peer but no open frame has yet been received.
    OpenSent,

    /// In this state the connection headers have been exchanged. An open frame, any pipelined
    /// connection traffic, and the close frame have been sent but no open frame has yet been
    /// received from the peer.
    ClosePipe,

    /// In this state the connection header and the open frame have been both sent and received.
    Opened,

    /// In this state a close frame has been received indicating that the peer has initiated an AMQP
    /// close. No further frames are expected to arrive on the connection; however, frames can still
    /// be sent. If desired, an implementation MAY do a TCP half-close at this point to shut down
    /// the read side of the connection.
    CloseReceived,

    /// In this state a close frame has been sent to the peer. It is illegal to write anything more
    /// onto the connection, however there could potentially still be incoming frames. If desired,
    /// an implementation MAY do a TCP half-close at this point to shutdown the write side of the
    /// connection.
    CloseSent,

    /// The DISCARDING state is a variant of the CLOSE SENT state where the close is triggered
    /// by an error. In this case any incoming frames on the connection MUST be silently discarded
    /// until the peer’s close frame is received.
    Discarding,

    /// In this state it is illegal for either endpoint to write anything more onto the connection. The
    /// connection can be safely closed and discarded.
    End,
}

/// A handle to 
pub struct ConnectionHandle {
    pub(crate) control: Sender<ConnectionControl>,
    handle: JoinHandle<Result<(), Error>>,

    // outgoing channel for session
    pub(crate) outgoing: Sender<SessionFrame>,
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        let _ = self.control.try_send(ConnectionControl::Close(None));
    }
}

impl ConnectionHandle {
    /// Checks if the underlying event loop has stopped
    pub fn is_closed(&self) -> bool {
        self.control.is_closed()
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<(), Error> {
        // If sending is unsuccessful, the `ConnectionEngine` event loop is
        // already dropped, this should be reflected by `JoinError` then.
        let _ = self.control.send(ConnectionControl::Close(None)).await;
        self.on_close().await
    }

    /// Close the connection with an error
    pub async fn close_with_error(
        &mut self,
        error: impl Into<definitions::Error>,
    ) -> Result<(), Error> {
        // If sending is unsuccessful, the `ConnectionEngine` event loop is
        // already dropped, this should be reflected by `JoinError` then.
        let _ = self
            .control
            .send(ConnectionControl::Close(Some(error.into())))
            .await;
        self.on_close().await
    }

    /// Returns when the underlying event loop has stopped
    ///
    /// # Panics
    ///
    /// Panics if calling `on_close` after executing any of [`close`] [`close_with_error`] or [`on_close`].
    /// This will cause the JoinHandle to be polled after completion, which causes a panic.
    pub async fn on_close(&mut self) -> Result<(), Error> {
        match (&mut self.handle).await {
            Ok(res) => res,
            Err(e) => Err(Error::JoinError(e)),
        }
    }

    pub(crate) async fn allocate_session(
        &mut self,
        tx: Sender<SessionIncomingItem>,
    ) -> Result<(u16, SessionId), AllocSessionError> {
        let (responder, resp_rx) = oneshot::channel();
        self.control
            .send(ConnectionControl::AllocateSession { tx, responder })
            .await?; // std::io::Error
        let result = resp_rx.await.map_err(|_| {
            AllocSessionError::Io(
                // The sending half is already dropped
                io::Error::new(
                    io::ErrorKind::Other,
                    "ConnectionEngine event_loop is dropped",
                ),
            )
        })?;
        result
    }

    // pub(crate) async fn drop_session(&mut self, session_id: SessionId) -> Result<(), Error> {
    //     self.control.send(ConnectionControl::DropSession(session_id)).await?;
    //     Ok(())
    // }
}

#[derive(Debug)]
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
    pub fn builder<'a>() -> builder::Builder<'a, WithoutContainerId> {
        builder::Builder::new()
    }

    pub async fn open(
        container_id: impl Into<String>, // TODO: default container id? random uuid-ish
        max_frame_size: impl Into<MaxFrameSize>, // TODO: make this use default?
        channel_max: impl Into<ChannelMax>, // make this use default?
        url: impl TryInto<Url, Error = url::ParseError>,
    ) -> Result<ConnectionHandle, OpenError> {
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
    type AllocError = AllocSessionError;
    type Error = Error;
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

    fn allocate_session(
        &mut self,
        tx: Sender<SessionIncomingItem>,
    ) -> Result<(u16, usize), Self::AllocError> {
        match &self.local_state {
            ConnectionState::Start
            | ConnectionState::HeaderSent
            | ConnectionState::HeaderReceived
            | ConnectionState::HeaderExchange
            | ConnectionState::CloseSent
            | ConnectionState::Discarding
            | ConnectionState::End => return Err(AllocSessionError::IllegalState),
            // TODO: what about pipelined open?
            _ => {}
        };

        // get new entry index
        let entry = self.local_sessions.vacant_entry();
        let session_id = entry.key();

        // check if there is enough
        if session_id > self.agreed_channel_max as usize {
            return Err(AllocSessionError::ChannelMaxReached);
        } else {
            entry.insert(tx);
            let channel = session_id as u16; // TODO: a different way of allocating session id?
            self.session_by_outgoing_channel.insert(channel, session_id);
            Ok((channel, session_id))
        }
    }

    fn deallocate_session(&mut self, session_id: usize) {
        self.local_sessions.remove(session_id);
    }

    /// Reacting to remote Open frame
    #[instrument(name = "RECV", skip_all)]
    async fn on_incoming_open(&mut self, channel: u16, open: Open) -> Result<(), Self::Error> {
        trace!(channel, frame = ?open);
        match &self.local_state {
            ConnectionState::HeaderExchange => self.local_state = ConnectionState::OpenReceived,
            ConnectionState::OpenSent => self.local_state = ConnectionState::Opened,
            ConnectionState::ClosePipe => self.local_state = ConnectionState::CloseSent,
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
        }

        // set channel_max to mutually acceptable
        self.agreed_channel_max = min(self.local_open.channel_max.0, open.channel_max.0);
        self.remote_open = Some(open);

        Ok(())
    }

    /// Reacting to remote Begin frame
    #[instrument(name = "RECV", skip_all)]
    async fn on_incoming_begin(&mut self, channel: u16, begin: Begin) -> Result<(), Self::Error> {
        trace!(channel, frame = ?begin);
        match &self.local_state {
            ConnectionState::Opened => {}
            // TODO: what about pipelined
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)), // TODO: what to do?
        }

        match begin.remote_channel {
            Some(outgoing_channel) => {
                let session_id = self
                    .session_by_outgoing_channel
                    .get(&outgoing_channel)
                    .ok_or_else(|| Error::amqp_error(AmqpError::NotFound, None))?; // Close with error NotFound

                if self.session_by_incoming_channel.contains_key(&channel) {
                    return Err(Error::amqp_error(AmqpError::NotAllowed, None)); // TODO: this is probably not how not allowed should be used?
                }
                self.session_by_incoming_channel
                    .insert(channel, *session_id);

                // forward begin to session
                let tx = self
                    .local_sessions
                    .get_mut(*session_id)
                    .ok_or_else(|| Error::amqp_error(AmqpError::NotFound, None))?;
                let sframe = SessionFrame::new(channel, SessionFrameBody::Begin(begin));
                tx.send(sframe).await?;
            }
            None => {
                // If a session is locally initiated, the remote-channel MUST NOT be set. When an endpoint responds
                // to a remotely initiated session, the remote-channel MUST be set to the channel on which the
                // remote session sent the begin.
                // TODO: allow remotely initiated session
                return Err(Error::amqp_error (
                    AmqpError::NotImplemented,
                    Some("Remotely initiazted session is not supported yet".to_string()),
                )); // Close with error NotImplemented
            }
        }

        Ok(())
    }

    /// Reacting to remote End frame
    #[instrument(name = "RECV", skip_all)]
    async fn on_incoming_end(&mut self, channel: u16, end: End) -> Result<(), Self::Error> {
        trace!(channel, frame = ?end);
        match &self.local_state {
            ConnectionState::Opened => {}
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
        }

        // Forward to session
        let sframe = SessionFrame::new(channel, SessionFrameBody::End(end));
        // Drop incoming channel
        let session_id = self
            .session_by_incoming_channel
            .remove(&channel)
            .ok_or_else(|| Error::amqp_error(AmqpError::NotFound, None))?;
        self.local_sessions
            .get_mut(session_id)
            .ok_or_else(|| Error::amqp_error(AmqpError::NotFound, None))?
            .send(sframe)
            .await?;

        Ok(())
    }

    /// Reacting to remote Close frame
    #[instrument(name = "RECV", skip_all)]
    async fn on_incoming_close(
        &mut self,
        channel: u16,
        close: Close,
    ) -> Result<(), Self::Error> {
        trace!(channel, frame=?close);

        match &self.local_state {
            ConnectionState::Opened => {
                self.local_state = ConnectionState::CloseReceived;
                self.control.send(ConnectionControl::Close(None)).await?;
            }
            ConnectionState::CloseSent => self.local_state = ConnectionState::End,
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
        };

        match close.error {
            Some(error) => Err(Error::Remote(error)),
            None => Ok(())
        }
    }

    #[instrument(name = "SEND", skip_all)]
    async fn send_open<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<Frame> + Send + Unpin,
        W::Error: Into<Error>,
    {
        let body = FrameBody::Open(self.local_open.clone());
        let frame = Frame::new(0u16, body);
        trace!(channel = 0, frame = ?frame.body);
        writer.send(frame).await.map_err(Into::into)?;

        // change local state after successfully sending the frame
        match &self.local_state {
            ConnectionState::HeaderExchange => self.local_state = ConnectionState::OpenSent,
            ConnectionState::OpenReceived => self.local_state = ConnectionState::Opened,
            ConnectionState::HeaderSent => self.local_state = ConnectionState::OpenPipe,
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
        }

        Ok(())
    }

    fn on_outgoing_begin(&mut self, channel: u16, begin: Begin) -> Result<Frame, Self::Error> {
        // TODO: the engine already checks that
        // match &self.local_state {
        //     ConnectionState::Opened => {}
        //     _ => return Err(Error::Message("Illegal local connection state")),
        // }

        let frame = Frame::new(channel, FrameBody::Begin(begin));
        Ok(frame)
    }

    #[instrument(skip_all)]
    fn on_outgoing_end(&mut self, channel: u16, end: End) -> Result<Frame, Self::Error> {
        self.session_by_outgoing_channel
            .remove(&channel)
            .ok_or_else(|| Error::amqp_error(AmqpError::NotFound, None))?;
        let frame = Frame::new(channel, FrameBody::End(end));
        Ok(frame)
    }

    // TODO: set a timeout for recving incoming Close
    #[instrument(name = "SEND", skip_all)]
    async fn send_close<W>(
        &mut self,
        writer: &mut W,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<Frame> + Send + Unpin,
        W::Error: Into<Error>,
    {
        let frame = Frame::new(0u16, FrameBody::Close(Close { error }));
        trace!(channel=0, frame = ?frame.body);
        writer.send(frame).await.map_err(Into::into)?;

        match &self.local_state {
            ConnectionState::Opened => self.local_state = ConnectionState::CloseSent,
            ConnectionState::CloseReceived => self.local_state = ConnectionState::End,
            ConnectionState::OpenSent => self.local_state = ConnectionState::ClosePipe,
            ConnectionState::OpenPipe => self.local_state = ConnectionState::OpenClosePipe,
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
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
