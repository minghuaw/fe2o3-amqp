//! Connection Listener

use std::{io, marker::PhantomData, time::Duration};


use fe2o3_amqp_types::{
    definitions::{self},
    performatives::{Begin, Close, End, Open},
    sasl::{SaslCode, SaslOutcome},
    states::ConnectionState,
};
use futures_util::{Sink, SinkExt, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf},
    sync::mpsc::{self, Receiver},
};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::{
    acceptor::sasl_acceptor::SaslServerFrame,
    connection::{
        self, engine::ConnectionEngine, ConnectionHandle, OpenError, DEFAULT_CONTROL_CHAN_BUF,
    },
    endpoint::{self, IncomingChannel, OutgoingChannel},
    frames::{
        amqp::{self, Frame},
        sasl,
    },
    session::frame::{SessionFrame, SessionFrameBody},
    transport::{protocol_header::ProtocolHeaderCodec, Transport},
    util::{Initialized, Uninitialized},
};

use super::{
    builder::Builder,
    sasl_acceptor::{SaslAcceptor, SaslAcceptorExt},
    IncomingSession,
};

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
/// use fe2o3_amqp::acceptor::ConnectionAcceptor;
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
/// use fe2o3_amqp::acceptor::ConnectionAcceptor;
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
/// ```rust,ignore
/// use fe2o3_amqp::acceptor::ConnectionAcceptor;
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
/// ```rust,ignore
/// use std::sync::Arc;
/// use fe2o3_amqp::acceptor::ConnectionAcceptor;
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
/// use fe2o3_amqp::acceptor::{ConnectionAcceptor, SaslPlainMechanism};
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
        framed_write: FramedWrite<WriteHalf<Io>, ProtocolHeaderCodec>,
        framed_read: FramedRead<ReadHalf<Io>, ProtocolHeaderCodec>,
    ) -> Result<ListenerConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        let mut local_state = ConnectionState::Start;
        let idle_timeout = self
            .local_open
            .idle_time_out
            .map(|millis| Duration::from_millis(millis as u64));
        let transport = Transport::negotiate_amqp_header(
            framed_write,
            framed_read,
            &mut local_state,
            idle_timeout,
        )
        .await?;

        let (control_tx, control_rx) = mpsc::channel(DEFAULT_CONTROL_CHAN_BUF);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(self.buffer_size);
        let (begin_tx, begin_rx) = mpsc::channel(self.buffer_size);

        let connection = connection::Connection::new(local_state, self.local_open.clone());
        let listener_connection = ListenerConnection {
            connection,
            session_listener: begin_tx,
        };

        let engine =
            ConnectionEngine::open(transport, listener_connection, control_rx, outgoing_rx).await?;
        let (handle, outcome) = engine.spawn();

        let connection_handle = ConnectionHandle {
            is_closed: false,
            control: control_tx,
            handle,
            outcome,
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
        let (reader, writer) = tokio::io::split(stream);
        let framed_write = FramedWrite::new(writer, ProtocolHeaderCodec::new());
        let framed_read = FramedRead::new(reader, ProtocolHeaderCodec::new());
        self.negotiate_amqp_with_framed(framed_write, framed_read)
            .await
    }
}

impl<Tls, Sasl> ConnectionAcceptor<Tls, Sasl>
where
    Sasl: SaslAcceptor,
{
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn negotiate_sasl_with_framed<Io>(
        &self,
        framed_write: FramedWrite<WriteHalf<Io>, ProtocolHeaderCodec>,
        framed_read: FramedRead<ReadHalf<Io>, ProtocolHeaderCodec>,
    ) -> Result<ListenerConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        let mut transport = Transport::negotiate_sasl_header(framed_write, framed_read).await?;

        // Send mechanisms
        let frame = sasl::Frame::Mechanisms(self.sasl_acceptor.sasl_mechanisms());
        #[cfg(feature = "tracing")]
        tracing::trace!(sending = ?frame);
        #[cfg(feature = "log")]
        log::trace!("sending = {:?}", frame);
        transport.send(frame).await?;

        let mut sasl_acceptor = self.sasl_acceptor.clone();
        loop {
            let frame = match transport.next().await.ok_or_else(|| {
                OpenError::Io(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Expecting SASL frames",
                ))
            })?? {
                sasl::Frame::Init(init) => sasl_acceptor.on_init(init),
                sasl::Frame::Response(response) => sasl_acceptor.on_response(response),
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
            };

            match frame {
                SaslServerFrame::Challenge(challenge) => {
                    let frame = sasl::Frame::Challenge(challenge);
                    #[cfg(feature = "tracing")]
                    tracing::trace!(sending = ?frame);
                    #[cfg(feature = "log")]
                    log::trace!("sending = {:?}", frame);
                    transport.send(frame).await?;
                }
                SaslServerFrame::Outcome(outcome) => {
                    let frame = sasl::Frame::Outcome(outcome);
                    #[cfg(feature = "tracing")]
                    tracing::trace!(sending = ?frame);
                    #[cfg(feature = "log")]
                    log::trace!("sending = {:?}", frame);
                    transport.send(frame).await?;
                    break;
                }
            }
        }

        // NOTE: LengthDelimitedCodec itself doesn't seem to carry any buffer, so
        // it should be fine to simply drop it.
        let (framed_write, framed_read) = transport.into_framed_codec();
        let framed_write = framed_write.map_encoder(|_| ProtocolHeaderCodec::new());
        let framed_read = framed_read.map_decoder(|_| ProtocolHeaderCodec::new());
        self.negotiate_amqp_with_framed(framed_write, framed_read)
            .await
    }

    async fn negotiate_sasl_with_stream<Io>(
        &self,
        stream: Io,
    ) -> Result<ListenerConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        let (reader, writer) = tokio::io::split(stream);
        let framed_write = FramedWrite::new(writer, ProtocolHeaderCodec::new());
        let framed_read = FramedRead::new(reader, ProtocolHeaderCodec::new());
        self.negotiate_sasl_with_framed(framed_write, framed_read)
            .await
    }
}

// A macro is used instead of blanked impl with trait to avoid heap allocated future
#[cfg(any(feature = "rustls", feature = "native-tls"))]
macro_rules! connect_tls {
    ($fn_ident:ident, $next_proto_header_handler:ident) => {
        async fn $fn_ident<Io>(&self, mut stream: Io) -> Result<ListenerConnectionHandle, OpenError>
        where
            Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
        {
            use crate::transport::protocol_header::ProtocolHeader;
            use tokio::io::AsyncWriteExt;

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

cfg_native_tls! {
    impl ConnectionAcceptor<tokio_native_tls::TlsAcceptor, ()> {
        connect_tls!(negotiate_tls_with_native_tls, negotiate_amqp_with_stream);
    }
    
    impl<Sasl> ConnectionAcceptor<tokio_native_tls::TlsAcceptor, Sasl>
    where
        Sasl: SaslAcceptor,
    {
        connect_tls!(negotiate_tls_with_native_tls, negotiate_sasl_with_stream);
    }
}

cfg_rustls! {
    impl ConnectionAcceptor<tokio_rustls::TlsAcceptor, ()> {
        connect_tls!(negotiate_tls_with_rustls, negotiate_amqp_with_stream);
    }
    
    impl<Sasl> ConnectionAcceptor<tokio_rustls::TlsAcceptor, Sasl>
    where
        Sasl: SaslAcceptor,
    {
        connect_tls!(negotiate_tls_with_rustls, negotiate_sasl_with_stream);
    }
}


impl ConnectionAcceptor<(), ()> {
    /// Accepts an incoming connection
    pub async fn accept<Io>(&self, stream: Io) -> Result<ListenerConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        self.negotiate_amqp_with_stream(stream).await
    }
}

impl<Sasl> ConnectionAcceptor<(), Sasl>
where
    Sasl: SaslAcceptor,
{
    /// Accepts an incoming connection
    pub async fn accept<Io>(&self, stream: Io) -> Result<ListenerConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        self.negotiate_sasl_with_stream(stream).await
    }
}

cfg_native_tls! {
    impl ConnectionAcceptor<tokio_native_tls::TlsAcceptor, ()> {
        /// Accepts an incoming connection
        pub async fn accept<Io>(&self, stream: Io) -> Result<ListenerConnectionHandle, OpenError>
        where
            Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
        {
            self.negotiate_tls_with_native_tls(stream).await
        }
    }
    
    impl<Sasl> ConnectionAcceptor<tokio_native_tls::TlsAcceptor, Sasl>
    where
        Sasl: SaslAcceptor,
    {
        /// Accepts an incoming connection
        pub async fn accept<Io>(&self, stream: Io) -> Result<ListenerConnectionHandle, OpenError>
        where
            Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
        {
            self.negotiate_tls_with_native_tls(stream).await
        }
    }
}

cfg_rustls! {
    impl ConnectionAcceptor<tokio_rustls::TlsAcceptor, ()> {
        /// Accepts an incoming connection
        pub async fn accept<Io>(&self, stream: Io) -> Result<ListenerConnectionHandle, OpenError>
        where
            Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
        {
            self.negotiate_tls_with_rustls(stream).await
        }
    }
    
    impl<Sasl> ConnectionAcceptor<tokio_rustls::TlsAcceptor, Sasl>
    where
        Sasl: SaslAcceptor,
    {
        /// Accepts an incoming connection
        pub async fn accept<Io>(&self, stream: Io) -> Result<ListenerConnectionHandle, OpenError>
        where
            Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
        {
            self.negotiate_tls_with_rustls(stream).await
        }
    }
}

/// A connection on the listener side
#[derive(Debug)]
pub struct ListenerConnection {
    pub(crate) connection: connection::Connection,
    pub(crate) session_listener: mpsc::Sender<IncomingSession>,
}


impl endpoint::Connection for ListenerConnection {
    type AllocError = <connection::Connection as endpoint::Connection>::AllocError;
    type OpenError = <connection::Connection as endpoint::Connection>::OpenError;
    type CloseError = <connection::Connection as endpoint::Connection>::CloseError;
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
    fn on_incoming_open(
        &mut self,
        channel: IncomingChannel,
        open: Open,
    ) -> Result<(), Self::OpenError> {
        self.connection.on_incoming_open(channel, open)
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
                    .map_err(|_| Self::Error::NotImplemented(None))?;
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
    fn on_incoming_close(
        &mut self,
        channel: IncomingChannel,
        close: Close,
    ) -> Result<(), Self::CloseError> {
        self.connection.on_incoming_close(channel, close)
    }

    #[inline]
    async fn send_open<W>(&mut self, writer: &mut W) -> Result<(), Self::OpenError>
    where
        W: Sink<Frame> + Send + Unpin,
        Self::OpenError: From<W::Error>,
    {
        self.connection.send_open(writer).await
    }

    #[inline]
    async fn send_close<W>(
        &mut self,
        writer: &mut W,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::CloseError>
    where
        W: Sink<Frame> + Send + Unpin,
        Self::CloseError: From<W::Error>,
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
                    Self::Error::NotFound(Some(String::from("Outgoing channel is not found")))
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
