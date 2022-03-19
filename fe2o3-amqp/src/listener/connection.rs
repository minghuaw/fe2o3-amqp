//! Connection Listener

use std::time::Duration;

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{self, AmqpError, Milliseconds, MIN_MAX_FRAME_SIZE},
    performatives::{Begin, Close, End, MaxFrameSize, Open},
    states::ConnectionState,
};
use futures_util::{sink::With, Sink};
use tokio::{io::AsyncReadExt, sync::mpsc};

use crate::{
    connection::{AllocSessionError, Error, OpenError, WithContainerId},
    endpoint,
    frames::amqp::{self, Frame},
    session::frame::{SessionFrame, SessionFrameBody},
    transport::{
        protocol_header::{ProtocolHeader, ProtocolId},
        Transport,
    },
    Connection,
};

use super::Listener;

type ConnectionBuilder<'a, Tls> = crate::connection::Builder<'a, WithContainerId, Tls>;

/// Listener for incoming connections
pub struct ConnectionListener<L: Listener> {
    idle_time_out: Option<Milliseconds>,
    listener: L,
}

impl<L: Listener> std::fmt::Debug for ConnectionListener<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionListener")
            // .field("builder", &self.builder)
            // .field("listener", &self.listener)
            .finish()
    }
}

impl<L> ConnectionListener<L>
where
    L: Listener,
{
    // /// Bind to a stream listener
    // pub fn bind(listener: L) -> Self {
    //     Self {

    //         listener
    //     }
    // }

    /// Accepts incoming connection
    pub async fn accept(&mut self) -> Result<(), OpenError> {
        let mut stream = self.listener.accept().await?;

        // Read protocol header
        let mut buf = [0u8; 8];
        stream.read_exact(&mut buf).await?;

        let header = match ProtocolHeader::try_from(buf) {
            Ok(header) => header,
            Err(buf) => {
                // Write protocol header and then disconnect the stream
                todo!()
            }
        };

        match header.id {
            ProtocolId::Amqp => {
                let idle_time_out = self
                    .idle_time_out
                    .map(|millis| Duration::from_millis(millis as u64));
                let transport =
                    Transport::<_, amqp::Frame>::bind(stream, MIN_MAX_FRAME_SIZE, idle_time_out);
            }
            ProtocolId::Tls => todo!(),
            ProtocolId::Sasl => todo!(),
        }

        todo!()
    }
}

/// A connection on the listener side
#[derive(Debug)]
pub struct ListenerConnection {
    connection: Connection,
    session_listener: mpsc::Sender<Begin>,
}

#[async_trait]
impl endpoint::Connection for ListenerConnection {
    type AllocError = <Connection as endpoint::Connection>::AllocError;

    type Error = <Connection as endpoint::Connection>::Error;

    type State = <Connection as endpoint::Connection>::State;

    type Session = <Connection as endpoint::Connection>::Session;
    #[inline]
    fn local_state(&self) -> &Self::State {
        self.connection.local_state()
    }
    #[inline]
    fn local_state_mut(&mut self) -> &mut Self::State {
        self.connection.local_state_mut()
    }

    #[inline]
    fn local_open(&self) -> &fe2o3_amqp_types::performatives::Open {
        self.connection.local_open()
    }

    #[inline]
    fn allocate_session(
        &mut self,
        tx: mpsc::Sender<crate::session::frame::SessionIncomingItem>,
    ) -> Result<(u16, crate::connection::engine::SessionId), Self::AllocError> {
        self.connection.allocate_session(tx)
    }

    #[inline]
    fn deallocate_session(&mut self, session_id: usize) {
        self.connection.deallocate_session(session_id)
    }

    #[inline]
    async fn on_incoming_open(&mut self, channel: u16, open: Open) -> Result<(), Self::Error> {
        self.connection.on_incoming_open(channel, open).await
    }

    #[inline]
    async fn on_incoming_begin(&mut self, channel: u16, begin: Begin) -> Result<(), Self::Error> {
        // This should remain mostly the same
        match self.connection.on_incoming_begin_inner(channel, &begin)? {
            Some(session_id) => {
                // forward begin to session
                let sframe = SessionFrame::new(channel, SessionFrameBody::Begin(begin));
                self.connection.send_to_session(session_id, sframe).await?;
            }
            None => {
                // If a session is locally initiated, the remote-channel MUST NOT be set. When an endpoint responds
                // to a remotely initiated session, the remote-channel MUST be set to the channel on which the
                // remote session sent the begin.
                // TODO: allow remotely initiated session

                // Upon receiving the
                // begin the partner will check the remote-channel field and find it empty. This indicates that the begin is referring to
                // remotely initiated session. The partner will therefore allocate an unused outgoing channel for the remotely initiated
                // session and indicate this by sending its own begin setting the remote-channel field to the incoming channel of the
                // remotely initiated session

                // Here we will send the begin frame out to get processed
                self.session_listener.send(begin).await.map_err(|_| {
                    Error::amqp_error(
                        AmqpError::NotImplemented,
                        Some("Remotely initiazted session is not supported".to_string()),
                    )
                })?;
            }
        }

        Ok(())
    }

    #[inline]
    async fn on_incoming_end(&mut self, channel: u16, end: End) -> Result<(), Self::Error> {
        self.connection.on_incoming_end(channel, end).await
    }

    #[inline]
    async fn on_incoming_close(&mut self, channel: u16, close: Close) -> Result<(), Self::Error> {
        self.connection.on_incoming_close(channel, close).await
    }

    #[inline]
    async fn send_open<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<Frame> + Send + Unpin,
        W::Error: Into<Self::Error>,
    {
        self.connection.send_open(writer).await
    }

    #[inline]
    async fn send_close<W>(
        &mut self,
        writer: &mut W,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<Frame> + Send + Unpin,
        W::Error: Into<Self::Error>,
    {
        self.connection.send_close(writer, error).await
    }

    #[inline]
    fn on_outgoing_begin(
        &mut self,
        channel: u16,
        begin: Begin,
    ) -> Result<amqp::Frame, Self::Error> {
        self.connection.on_outgoing_begin(channel, begin)
    }

    #[inline]
    fn on_outgoing_end(
        &mut self,
        channel: u16,
        end: fe2o3_amqp_types::performatives::End,
    ) -> Result<amqp::Frame, Self::Error> {
        self.connection.on_outgoing_end(channel, end)
    }

    #[inline]
    fn session_tx_by_incoming_channel(
        &mut self,
        channel: u16,
    ) -> Option<&mut mpsc::Sender<crate::session::frame::SessionIncomingItem>> {
        self.connection.session_tx_by_incoming_channel(channel)
    }

    #[inline]
    fn session_tx_by_outgoing_channel(
        &mut self,
        channel: u16,
    ) -> Option<&mut mpsc::Sender<crate::session::frame::SessionIncomingItem>> {
        self.connection.session_tx_by_outgoing_channel(channel)
    }
}
