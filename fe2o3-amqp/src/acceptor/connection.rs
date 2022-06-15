//! Connection Listener

use std::{io, marker::PhantomData, time::Duration};

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{self, AmqpError, MIN_MAX_FRAME_SIZE},
    performatives::{Begin, Close, End, Open},
    primitives::Symbol,
    sasl::{SaslCode, SaslMechanisms, SaslOutcome},
    states::ConnectionState,
};
use futures_util::{Sink, SinkExt, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::mpsc::{self, Receiver},
};
use tokio_util::codec::Framed;

use crate::{
    acceptor::sasl_acceptor::SaslServerFrame,
    connection::{
        self, engine::ConnectionEngine, ConnectionHandle, Error, OpenError,
        DEFAULT_CONTROL_CHAN_BUF,
    },
    endpoint::{self, IncomingChannel, OutgoingChannel},
    frames::{
        amqp::{self, Frame},
        sasl,
    },
    session::frame::{SessionFrame, SessionFrameBody},
    transport::{
        protocol_header::{ProtocolHeader, ProtocolHeaderCodec},
        send_amqp_proto_header, Transport,
    },
    util::{Initialized, Uninitialized},
};

use super::{builder::Builder, sasl_acceptor::SaslAcceptor, IncomingSession};

/// Type alias for listener connection handle
pub type ListenerConnectionHandle = ConnectionHandle<Receiver<IncomingSession>>;

impl ListenerConnectionHandle {
    /// Waits for the next incoming session asynchronously
    pub async fn next_incoming_session(&mut self) -> Option<IncomingSession> {
        self.session_listener.recv().await
    }
}

/// Acceptor for an incoming connection
///
/// # Accepts incoming connection with the default configuration.
///
/// ```rust, ignore
/// use tokio::net::TcpListener;
/// use crate::acceptor::ConnectionAcceptor;
///
/// let tcp_listener = TcpListener::bind("localhost:5672").await.unwrap();
/// let connection_acceptor = ConnectionAcceptor::new("example-listener");
///
/// if let Ok((stream, addr)) = tcp_listener.accept().await {
///     // Any type that implements `AsyncRead` and `AsyncWrite` can be used
///     let connection = connection_acceptor.accept(stream).await.unwrap();
/// }
/// ```
///
/// ## Default configuration
///
/// | Field | Default Value |
/// |-------|---------------|
/// |`max_frame_size`| [`crate::connection::DEFAULT_MAX_FRAME_SIZE`] |
/// |`channel_max`| [`crate::connection::DEFAULT_CHANNEL_MAX`] |
/// |`idle_time_out`| `None` |
/// |`outgoing_locales`| `None` |
/// |`incoming_locales`| `None` |
/// |`offered_capabilities`| `None` |
/// |`desired_capabilities`| `None` |
/// |`Properties`| `None` |
///
/// # Customize configuration
///
/// The [`ConnectionAcceptor`] can be configured either using the builder pattern
/// or modifying particular field after the acceptor is built.
///
/// ```rust
/// use crate::acceptor::ConnectionAcceptor;
///
/// let connection_acceptor = ConnectionAcceptor::builder()
///     .container_id("example-listener")
///     .max_frame_size(4096) // Customize max frame size
///     .build();
/// ```
///
/// # TLS Acceptor
///
/// TLS acceptor can be added to handle TLS negotiation.
///
/// ## `tokio-native-tls` (requires `"native-tls"` feature)
///
/// ```rust
/// use crate::acceptor::ConnectionAcceptor;
///
/// let tls_acceptor =
///     tokio_native_tls::TlsAcceptor::from(native_tls::TlsAcceptor::builder(cert).build()?);
/// let connection_acceptor = ConnectionAcceptor::builder()
///     .container_id("example-listener")
///     .tls_acceptor(tls_acceptor)
///     .build();
/// ```
///
/// ## `tokio-rustls` (requires `"rustls"` feature)
///
/// ```rust
/// use crate::acceptor::ConnectionAcceptor;
///
/// let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));;
/// let connection_acceptor = ConnectionAcceptor::builder()
///     .container_id("example-listener")
///     .tls_acceptor(tls_acceptor)
///     .build();
/// ```
///
/// # SASL Acceptor
///
/// Currently there is only one naive SASL acceptor implemented. Any type that
/// implements the [`SaslAcceptor`] trait would work.
///
/// ## Simple PLAIN SASL Acceptor
///
/// ```rust,ignore
/// use crate::acceptor::{ConnectionAcceptor, SaslPlainMechanism};
///
/// let connection_acceptor = ConnectionAcceptor::builder()
///     .container_id("example-listener")
///     .sasl_acceptor(SaslPlainMechanism::new("guest", "guest"))
///     .build();
/// ```
#[derive(Debug)]
pub struct ConnectionAcceptor<Tls, Sasl> {
    /// Local Open performative that holds the majority of configurable fields
    pub local_open: Open,

    /// TLS acceptor that handles TLS negotiation
    pub tls_acceptor: Tls,

    /// SASL acceptor that handles SASL negotiation
    pub sasl_acceptor: Sasl,

    /// Buffer size for the underlying channel
    pub buffer_size: usize,
}

impl ConnectionAcceptor<(), ()> {
    /// Creates a default [`ConnectionAcceptor`] with the supplied container id
    pub fn new(container_id: impl Into<String>) -> Self {
        Self::builder().container_id(container_id).build()
    }

    /// Creates a builder for [`ConnectionAcceptor`]
    pub fn builder() -> Builder<Self, Uninitialized> {
        Builder::<Self, Uninitialized>::new()
    }
}

impl<Tls, Sasl> ConnectionAcceptor<Tls, Sasl> {
    /// Convert the acceptor into a connection acceptor builder. This allows users changing
    /// particular field of the acceptor.
    pub fn into_builder(self) -> Builder<ConnectionAcceptor<Tls, Sasl>, Initialized> {
        Builder {
            inner: self,
            marker: PhantomData,
        }
    }

    async fn negotiate_amqp_with_framed<Io>(
        &self,
        framed: Framed<Io, ProtocolHeaderCodec>
    ) -> Result<ListenerConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        let mut local_state = ConnectionState::HeaderReceived;
        let idle_timeout = self
            .local_open
            .idle_time_out
            .map(|millis| Duration::from_millis(millis as u64));
        let transport = Transport::negotiate_amqp_header(framed, &mut local_state, idle_timeout).await?;

        let (control_tx, control_rx) = mpsc::channel(DEFAULT_CONTROL_CHAN_BUF);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(self.buffer_size);
        let (begin_tx, begin_rx) = mpsc::channel(self.buffer_size);

        let connection =
            connection::Connection::new(control_tx.clone(), local_state, self.local_open.clone());
        let listener_connection = ListenerConnection {
            connection,
            session_listener: begin_tx,
        };

        let engine =
            ConnectionEngine::open(transport, listener_connection, control_rx, outgoing_rx).await?;
        let handle = engine.spawn();

        let connection_handle = ConnectionHandle {
            control: control_tx,
            handle,
            outgoing: outgoing_tx,
            session_listener: begin_rx,
        };
        Ok(connection_handle)
    }

    async fn negotiate_amqp_with_stream<Io>(
        &self,
        stream: Io,
    ) -> Result<ListenerConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        let framed = Framed::new(stream, ProtocolHeaderCodec::new());
        self.negotiate_amqp_with_framed(framed).await
    }
}

impl<Tls, Sasl> ConnectionAcceptor<Tls, Sasl>
where
    Sasl: SaslAcceptor,
{
    async fn negotiate_sasl_with_framed<Io>(
        &self,
        framed: Framed<Io, ProtocolHeaderCodec>
    ) -> Result<ListenerConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        let mut transport = Transport::negotiate_sasl_header(framed).await?;

        // Send mechanisms
        let mechanisms = SaslMechanisms {
            sasl_server_mechanisms: self
                .sasl_acceptor
                .mechanisms()
                .into_iter()
                .map(|s| Symbol::from(s))
                .collect(),
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
                    return Err(OpenError::SaslError {
                        code: SaslCode::Sys,
                        additional_data: None,
                    });
                }
            }
        } else {
            return Err(OpenError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Expecting SASL Init frame",
            )));
        };

        let outcome: SaslOutcome = match next {
            SaslServerFrame::Challenge(challenge) => {
                transport.send(sasl::Frame::Challenge(challenge)).await?;
                self.negotiate_sasl_challenge(&mut transport).await?
            }
            SaslServerFrame::Outcome(outcome) => outcome,
        };

        transport.send(sasl::Frame::Outcome(outcome)).await?;
        
        // NOTE: LengthDelimitedCodec itself doesn't seem to carry any buffer, so
        // it should be fine to simply drop it.
        let framed = transport.into_framed_codec()
            .map_codec(|_| ProtocolHeaderCodec::new());
        self.negotiate_amqp_with_framed(framed).await
    }

    async fn negotiate_sasl_with_stream<Io>(
        &self,
        stream: Io,
    ) -> Result<ListenerConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        let framed = Framed::new(stream, ProtocolHeaderCodec::new());
        self.negotiate_sasl_with_framed(framed).await
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
                        }
                        SaslServerFrame::Outcome(outcome) => {
                            // The Ok result will be sent by the outer function
                            return Ok(outcome);
                        }
                    }
                }
                _ => {
                    let outcome = SaslOutcome {
                        code: SaslCode::Sys,
                        additional_data: None,
                    };
                    transport.send(sasl::Frame::Outcome(outcome)).await?;
                    return Err(OpenError::SaslError {
                        code: SaslCode::Sys,
                        additional_data: None,
                    });
                }
            }
        }

        Err(OpenError::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Expecting SASL response",
        )))
    }
}

// A macro is used instead of blanked impl with trait to avoid heap allocated future
#[cfg(any(feature = "rustls", feature = "native-tls"))]
macro_rules! connect_tls {
    ($fn_ident:ident, $next_proto_header_handler:ident) => {
        async fn $fn_ident<Io>(
            &self,
            mut stream: Io,
        ) -> Result<ListenerConnectionHandle, OpenError>
        where
            Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
        {
            let incoming_header = crate::transport::recv_tls_proto_header(&mut stream).await?;

            let tls_header = ProtocolHeader::tls();
            if tls_header != incoming_header {
                let buf: [u8; 8] = tls_header.into();
                stream.write_all(&buf).await?;
                return Err(OpenError::ProtocolHeaderMismatch(incoming_header.into()));
            }

            // Send protocol header
            let buf: [u8; 8] = tls_header.into();
            stream.write_all(&buf).await?;

            let tls_stream = self.tls_acceptor.accept(stream).await.map_err(|e| {
                OpenError::Io(io::Error::new(io::ErrorKind::Other, format!("{:?}", e)))
            })?;

            self.$next_proto_header_handler(tls_stream).await
        }
    };
}

#[cfg(feature = "native-tls")]
impl ConnectionAcceptor<tokio_native_tls::TlsAcceptor, ()> {
    connect_tls!(negotiate_tls_with_native_tls, negotiate_amqp_with_stream);
}

#[cfg(feature = "native-tls")]
impl<Sasl> ConnectionAcceptor<tokio_native_tls::TlsAcceptor, Sasl>
where
    Sasl: SaslAcceptor,
{
    connect_tls!(negotiate_tls_with_native_tls, negotiate_sasl_with_stream);
}

#[cfg(feature = "rustls")]
impl ConnectionAcceptor<tokio_rustls::TlsAcceptor, ()> {
    connect_tls!(negotiate_tls_with_rustls, negotiate_amqp_with_stream);
}

#[cfg(feature = "rustls")]
impl<Sasl> ConnectionAcceptor<tokio_rustls::TlsAcceptor, Sasl>
where
    Sasl: SaslAcceptor,
{
    connect_tls!(negotiate_tls_with_rustls, negotiate_sasl_with_stream);
}

macro_rules! impl_accept {
    (<$tls:ty, $sasl:ty>, $proto_header_handler:ident) => {
        /// Accepts an incoming connection
        pub async fn accept<Io>(
            &self,
            mut stream: Io,
        ) -> Result<ListenerConnectionHandle, OpenError>
        where
            Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
        {
            self.$proto_header_handler(stream).await
        }
    };
}

impl ConnectionAcceptor<(), ()> {
    impl_accept!(<(),()>, negotiate_amqp_with_stream);
}

impl<Sasl> ConnectionAcceptor<(), Sasl>
where
    Sasl: SaslAcceptor,
{
    impl_accept!(<(), Sasl>, negotiate_sasl_with_stream);
}

#[cfg(feature = "native-tls")]
impl ConnectionAcceptor<tokio_native_tls::TlsAcceptor, ()> {
    impl_accept!(<tokio_native_tls::TlsAcceptor, ()>, negotiate_tls_with_native_tls);
}

#[cfg(feature = "native-tls")]
impl<Sasl> ConnectionAcceptor<tokio_native_tls::TlsAcceptor, Sasl>
where
    Sasl: SaslAcceptor,
{
    impl_accept!(<tokio_native_tls::TlsAcceptor, Sasl>, negotiate_tls_with_native_tls);
}

#[cfg(feature = "rustls")]
impl ConnectionAcceptor<tokio_rustls::TlsAcceptor, ()> {
    impl_accept!(<tokio_rustls::TlsAcceptor, ()>, negotiate_tls_with_rustls);
}

#[cfg(feature = "rustls")]
impl<Sasl> ConnectionAcceptor<tokio_rustls::TlsAcceptor, Sasl>
where
    Sasl: SaslAcceptor,
{
    impl_accept!(<tokio_rustls::TlsAcceptor, Sasl>, negotiate_tls_with_rustls);
}

/// A connection on the listener side
#[derive(Debug)]
pub struct ListenerConnection {
    pub(crate) connection: connection::Connection,
    pub(crate) session_listener: mpsc::Sender<IncomingSession>,
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
    ) -> Result<OutgoingChannel, Self::AllocError> {
        self.connection.allocate_session(tx)
    }

    #[inline]
    fn deallocate_session(&mut self, outgoing_channel: OutgoingChannel) {
        self.connection.deallocate_session(outgoing_channel)
    }

    #[inline]
    async fn on_incoming_open(
        &mut self,
        channel: IncomingChannel,
        open: Open,
    ) -> Result<(), Self::Error> {
        self.connection.on_incoming_open(channel, open).await
    }

    #[inline]
    async fn on_incoming_begin(
        &mut self,
        channel: IncomingChannel,
        begin: Begin,
    ) -> Result<(), Self::Error> {
        // This should remain mostly the same
        match self.connection.on_incoming_begin_inner(channel, &begin)? {
            Some(relay) => {
                // forward begin to session
                let sframe = SessionFrame::new(channel, SessionFrameBody::Begin(begin));
                relay.send(sframe).await?;
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
                let incoming_session = IncomingSession {
                    channel: channel.0,
                    begin,
                };
                self.session_listener
                    .send(incoming_session)
                    .await
                    .map_err(|_| {
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
    async fn on_incoming_end(
        &mut self,
        channel: IncomingChannel,
        end: End,
    ) -> Result<(), Self::Error> {
        self.connection.on_incoming_end(channel, end).await
    }

    #[inline]
    async fn on_incoming_close(
        &mut self,
        channel: IncomingChannel,
        close: Close,
    ) -> Result<(), Self::Error> {
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
        outgoing_channel: OutgoingChannel,
        begin: Begin,
    ) -> Result<amqp::Frame, Self::Error> {
        if let Some(remote_channel) = begin.remote_channel {
            let relay = self
                .connection
                .session_by_outgoing_channel
                .get(outgoing_channel.0 as usize)
                .ok_or_else(|| {
                    Error::amqp_error(
                        AmqpError::InternalError,
                        "Outgoing channel is not found".to_string(),
                    )
                })?;

            self.connection
                .session_by_incoming_channel
                .insert(IncomingChannel(remote_channel), relay.clone());
        }

        self.connection.on_outgoing_begin(outgoing_channel, begin)
    }

    #[inline]
    fn on_outgoing_end(
        &mut self,
        channel: OutgoingChannel,
        end: fe2o3_amqp_types::performatives::End,
    ) -> Result<amqp::Frame, Self::Error> {
        self.connection.on_outgoing_end(channel, end)
    }

    #[inline]
    fn session_tx_by_incoming_channel(
        &mut self,
        channel: IncomingChannel,
    ) -> Option<&mpsc::Sender<crate::session::frame::SessionIncomingItem>> {
        self.connection.session_tx_by_incoming_channel(channel)
    }

    #[inline]
    fn session_tx_by_outgoing_channel(
        &mut self,
        channel: OutgoingChannel,
    ) -> Option<&mpsc::Sender<crate::session::frame::SessionIncomingItem>> {
        self.connection.session_tx_by_outgoing_channel(channel)
    }
}
