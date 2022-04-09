//! Connection Listener

use std::{time::Duration, io, marker::PhantomData};

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{self, AmqpError, MIN_MAX_FRAME_SIZE},
    performatives::{Begin, Close, End, Open},
    states::ConnectionState, sasl::{SaslMechanisms, SaslOutcome, SaslCode}, primitives::Symbol,
};
use futures_util::{Sink, SinkExt, StreamExt};
use tokio::{io::{AsyncReadExt, AsyncRead, AsyncWrite, AsyncWriteExt}, sync::mpsc::{self, Receiver}};

use crate::{
    connection::{Error, self, ConnectionHandle, OpenError, DEFAULT_CONTROL_CHAN_BUF, engine::ConnectionEngine},
    endpoint,
    frames::{amqp::{self, Frame}, sasl},
    session::frame::{SessionFrame, SessionFrameBody}, transport::{protocol_header::{ProtocolHeader, ProtocolId}, Transport, send_amqp_proto_header}, listener::sasl_acceptor::SaslServerFrame, util::Initialized,
};

use super::{sasl_acceptor::{SaslAcceptor}, builder::Builder};

/// Type alias for listener connection
pub type ListenerConnectionHandle = ConnectionHandle<Receiver<Begin>>;

// /// Listener for incoming connections
// pub struct ConnectionListener<L: Listener> {
//     idle_time_out: Option<Milliseconds>,
//     listener: L,
// }

// impl<L: Listener> std::fmt::Debug for ConnectionListener<L> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("ConnectionListener")
//             // .field("builder", &self.builder)
//             // .field("listener", &self.listener)
//             .finish()
//     }
// }

// impl ConnectionListener<TcpListener> {

// }

/// Acceptor for an incoming connection
#[derive(Debug)]
pub struct ConnectionAcceptor<Tls, Sasl> {
    pub(crate) local_open: Open,
    pub(crate) tls_acceptor: Tls,
    pub(crate) sasl_acceptor: Sasl,
    pub(crate) buffer_size: usize,
}

impl<Tls, Sasl> ConnectionAcceptor<Tls, Sasl> {
    /// Convert the acceptor into a connection acceptor builder. This allows users changing
    /// particular field of the acceptor.
    pub fn into_builder(self) -> Builder<ConnectionAcceptor<Tls,Sasl>, Initialized> {
        Builder {
            inner: self,
            marker: PhantomData
        }
    }

    async fn negotiate_amqp_with_stream<Io>(&self, mut stream: Io, incoming_header: ProtocolHeader) -> Result<ListenerConnectionHandle, OpenError> 
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        let mut local_state = ConnectionState::HeaderReceived;
        let idle_time_out = self.local_open
            .idle_time_out
            .map(|millis| Duration::from_millis(millis as u64));
        let local_header = ProtocolHeader::amqp();
        if incoming_header != local_header {
            let buf: [u8; 8] = local_header.into();
            stream.write_all(&buf).await?;
            return Err(OpenError::ProtocolHeaderMismatch(incoming_header.into()))
        }
        send_amqp_proto_header(&mut stream, &mut local_state, local_header).await?;
        let transport =
            Transport::<_, amqp::Frame>::bind(stream, MIN_MAX_FRAME_SIZE, idle_time_out);
    
        let (control_tx, control_rx) = mpsc::channel(DEFAULT_CONTROL_CHAN_BUF);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(self.buffer_size);
        let (begin_tx, begin_rx) = mpsc::channel(self.buffer_size);
        
        let connection = connection::Connection::new(control_tx.clone(), local_state, self.local_open.clone());
        let listener_connection = ListenerConnection {
            connection,
            session_listener: begin_tx
        };
    
        let engine = ConnectionEngine::open(
            transport,
            listener_connection,
            control_rx,
            outgoing_rx
        ).await?;
        let handle = engine.spawn();
    
        let connection_handle = ConnectionHandle {
            control: control_tx,
            handle,
            outgoing: outgoing_tx,
            session_listener: begin_rx
        };
        Ok(connection_handle)
    }
}

macro_rules! sasl_not_impled {
    ($header:expr) => {
        async fn sasl_not_implemented<Io>(&self, mut stream: Io, incoming_header: ProtocolHeader) -> Result<ListenerConnectionHandle, OpenError>
        where
            Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
        {
            let header = $header;
            let buf: [u8; 8] = header.into();
            stream.write_all(&buf).await?;
            Err(OpenError::ProtocolHeaderMismatch(incoming_header.into())) 
        }
    };
}

impl ConnectionAcceptor<(), ()> {
    sasl_not_impled!(ProtocolHeader::amqp());
}

#[cfg(feature = "native-tls")]
impl ConnectionAcceptor<tokio_native_tls::TlsAcceptor, ()> {
    sasl_not_impled!(ProtocolHeader::tls());
}

#[cfg(feature = "rustls")]
impl ConnectionAcceptor<tokio_rustls::TlsAcceptor, ()> {
    sasl_not_impled!(ProtocolHeader::tls());
}

macro_rules! tls_not_impled {
    ($header:expr) => {
        async fn tls_not_implemented<Io>(&self, mut stream: Io, incoming_header: ProtocolHeader) -> Result<ListenerConnectionHandle, OpenError>
        where
            Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
        {
            // Send back supported protocol header: AMQP or SASL
            let header = $header;
            let buf: [u8; 8] = header.into();
            stream.write_all(&buf).await?;
            Err(OpenError::ProtocolHeaderMismatch(incoming_header.into())) 
        }
    };
}

impl<Sasl> ConnectionAcceptor<(), Sasl> 
where
    Sasl: SaslAcceptor,
{
    tls_not_impled!(ProtocolHeader::sasl());
}

impl ConnectionAcceptor<(), ()> {
    tls_not_impled!(ProtocolHeader::amqp());
}

impl<Tls, Sasl> ConnectionAcceptor<Tls, Sasl>
where
    Sasl: SaslAcceptor,
{
    async fn negotiate_sasl_with_stream<Io>(&self, mut stream: Io, incoming_header: ProtocolHeader) -> Result<ListenerConnectionHandle, OpenError> 
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        let local_header = ProtocolHeader::sasl();
        if incoming_header != local_header {
            let buf: [u8; 8] = local_header.into();
            stream.write_all(&buf).await?;
            return Err(OpenError::ProtocolHeaderMismatch(incoming_header.into()))
        }

        let mut buf: [u8; 8] = local_header.into();
        stream.write_all(&buf).await?;

        let mut transport = Transport::<_, sasl::Frame>::bind(stream, MIN_MAX_FRAME_SIZE, None);

        // Send mechanisms
        let mechanisms = SaslMechanisms {
            sasl_server_mechanisms: self.sasl_acceptor.mechanisms()
                .into_iter()
                .map(|s| Symbol::from(s)).collect(),
        };
        transport.send(sasl::Frame::Mechanisms(mechanisms)).await?;

        // Wait for Init 
        let next = if let Some(frame) = transport.next().await {
            match frame? {
                sasl::Frame::Init(init) => self.sasl_acceptor.on_init(init),
                _ => {
                    let outcome = SaslOutcome {
                        code: SaslCode::Sys,
                        additional_data: None,
                    };
                    transport.send(sasl::Frame::Outcome(outcome)).await?;
                    return Err(OpenError::SaslError { code: SaslCode::Sys, additional_data: None })
                }
            }
        } else {
            return Err(OpenError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Expecting SASL Init frame"
            )))
        };

        let outcome: SaslOutcome = match next {
            SaslServerFrame::Challenge(challenge) => {
                transport.send(sasl::Frame::Challenge(challenge)).await?;
                self.negotiate_sasl_challenge(&mut transport).await?
            },
            SaslServerFrame::Outcome(outcome) => outcome,
        };

        transport.send(sasl::Frame::Outcome(outcome)).await?;

        let mut stream = transport.into_inner_io();
        stream.read_exact(&mut buf).await?;
        let incoming_header = ProtocolHeader::try_from(buf)
            .map_err(|v| OpenError::ProtocolHeaderMismatch(v))?;

        self.negotiate_amqp_with_stream(stream, incoming_header).await
    }

    async fn negotiate_sasl_challenge<Io>(
        &self,
        transport: &mut Transport<Io, sasl::Frame>,
    ) -> Result<SaslOutcome, OpenError> 
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        // Send initial challenge
        while let Some(frame) = transport.next().await {
            match frame? {
                sasl::Frame::Response(response) => {
                    match self.sasl_acceptor.on_response(response) {
                        SaslServerFrame::Challenge(challenge) => {
                            transport.send(sasl::Frame::Challenge(challenge)).await?;
                        },
                        SaslServerFrame::Outcome(outcome) => {
                            // The Ok result will be sent by the outer function
                            return Ok(outcome)
                        },
                    }
                },
                _ => {
                    let outcome = SaslOutcome {
                        code: SaslCode::Sys,
                        additional_data: None,
                    };
                    transport.send(sasl::Frame::Outcome(outcome)).await?;
                    return Err(OpenError::SaslError { code: SaslCode::Sys, additional_data: None })
                }
            }
        }

        Err(OpenError::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Expecting SASL response"
        )))
    }
}


// A macro is used instead of blanked impl with trait to avoid heap allocated future
macro_rules! connect_tls {
    ($fn_ident:ident, $sasl_handler:ident) => {
        async fn $fn_ident<Io>(&self, mut stream: Io, incoming_header: ProtocolHeader) -> Result<ListenerConnectionHandle, OpenError> 
        where
            Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
        {
            let tls_header = ProtocolHeader::tls();
            if tls_header != incoming_header {
                let buf: [u8; 8] = tls_header.into();
                stream.write_all(&buf).await?;
                return Err(OpenError::ProtocolHeaderMismatch(incoming_header.into()))
            }
            
            // Send protocol header
            let mut buf: [u8; 8] = tls_header.into();
            stream.write_all(&buf).await?;

            let mut tls_stream = self.tls_acceptor.accept(stream).await.map_err(|e| {
                OpenError::Io(io::Error::new(io::ErrorKind::Other, format!("{:?}", e)))
            })?;

            // Read AMQP header
            tls_stream.read_exact(&mut buf).await?;
            let incoming_header = ProtocolHeader::try_from(buf)
                .map_err(|v| OpenError::ProtocolHeaderMismatch(v))?;

            match incoming_header.id {
                ProtocolId::Amqp => self.negotiate_amqp_with_stream(tls_stream, incoming_header).await,
                ProtocolId::Tls => Err(OpenError::ProtocolHeaderMismatch(incoming_header.into())),
                ProtocolId::Sasl => self.$sasl_handler(tls_stream, incoming_header).await,
            }
        }
    };
}

#[cfg(feature = "native-tls")]
impl ConnectionAcceptor<tokio_native_tls::TlsAcceptor, ()> {
    connect_tls!(connect_tls_with_native_tls, sasl_not_implemented);
}

#[cfg(feature = "native-tls")]
impl<Sasl> ConnectionAcceptor<tokio_native_tls::TlsAcceptor, Sasl> 
where
    Sasl: SaslAcceptor,
{
    connect_tls!(connect_tls_with_native_tls, negotiate_sasl_with_stream);
}

#[cfg(feature = "rustls")]
impl ConnectionAcceptor<tokio_rustls::TlsAcceptor, ()> {
    connect_tls!(connect_tls_with_rustls, sasl_not_implemented);
}

#[cfg(feature = "rustls")]
impl<Sasl> ConnectionAcceptor<tokio_rustls::TlsAcceptor, Sasl> 
where
    Sasl: SaslAcceptor,
{
    connect_tls!(connect_tls_with_rustls, negotiate_sasl_with_stream);
}

macro_rules! impl_accept {
    (<$tls:ty, $sasl:ty>, $tls_handler:ident, $sasl_handler:ident) => {
        /// Accepts an incoming connection
        pub async fn accept<Io>(&self, mut stream: Io) -> Result<ListenerConnectionHandle, OpenError> 
        where
            Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
        {
            // Read protocol header
            let mut buf = [0u8; 8];
            stream.read_exact(&mut buf).await?;

            let incoming_header = ProtocolHeader::try_from(buf)
                .map_err(|v| OpenError::ProtocolHeaderMismatch(v))?;

            match incoming_header.id {
                ProtocolId::Amqp => {
                    self.negotiate_amqp_with_stream(stream, incoming_header).await
                }
                ProtocolId::Tls => {
                    self.$tls_handler(stream, incoming_header).await
                },
                ProtocolId::Sasl => self.$sasl_handler(stream, incoming_header).await,
            }
        }
    };
}

impl ConnectionAcceptor<(), ()> {
    impl_accept!(<(),()>, tls_not_implemented, sasl_not_implemented);
}

impl<Sasl> ConnectionAcceptor<(), Sasl>
where
    Sasl: SaslAcceptor,
{
    impl_accept!(<(), Sasl>, tls_not_implemented, negotiate_sasl_with_stream);
}

#[cfg(feature = "native-tls")]
impl ConnectionAcceptor<tokio_native_tls::TlsAcceptor, ()> {
    impl_accept!(<tokio_native_tls::TlsAcceptor, ()>, connect_tls_with_native_tls, sasl_not_implemented);
}

#[cfg(feature = "native-tls")]
impl<Sasl> ConnectionAcceptor<tokio_native_tls::TlsAcceptor, Sasl>
where
    Sasl: SaslAcceptor,
{
    impl_accept!(<tokio_native_tls::TlsAcceptor, Sasl>, connect_tls_with_native_tls, negotiate_sasl_with_stream);
}

#[cfg(feature = "rustls")]
impl ConnectionAcceptor<tokio_rustls::TlsAcceptor, ()> {
    impl_accept!(<tokio_rustls::TlsAcceptor, ()>, connect_tls_with_rustls, sasl_not_implemented);
}

#[cfg(feature = "rustls")]
impl<Sasl> ConnectionAcceptor<tokio_rustls::TlsAcceptor, Sasl>
where
    Sasl: SaslAcceptor,
{
    impl_accept!(<tokio_rustls::TlsAcceptor, Sasl>, connect_tls_with_rustls, negotiate_sasl_with_stream);
}

/// A connection on the listener side
#[derive(Debug)]
pub struct ListenerConnection {
    pub(crate) connection: connection::Connection,
    pub(crate) session_listener: mpsc::Sender<Begin>,
}

#[async_trait]
impl endpoint::Connection for ListenerConnection {
    type AllocError = <connection::Connection as endpoint::Connection>::AllocError;

    type Error = <connection::Connection as endpoint::Connection>::Error;

    type State = <connection::Connection as endpoint::Connection>::State;

    type Session = <connection::Connection as endpoint::Connection>::Session;

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

