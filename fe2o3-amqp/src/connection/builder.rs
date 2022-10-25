//! Builder for [`crate::Connection`]

use std::{convert::TryInto, io, marker::PhantomData, time::Duration};

use fe2o3_amqp_types::{
    definitions::{Fields, IetfLanguageTag, Milliseconds, MIN_MAX_FRAME_SIZE},
    performatives::{ChannelMax, MaxFrameSize, Open},
    sasl::SaslCode,
};
use futures_util::{SinkExt, StreamExt};
use serde_amqp::primitives::Symbol;
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::mpsc::{self},
};
use tokio_util::codec::{FramedRead, FramedWrite};
use url::Url;

use crate::{
    connection::{Connection, ConnectionState},
    frames::sasl,
    sasl_profile::{Negotiation, SaslProfile},
    transport::Transport,
    transport::{error::NegotiationError, protocol_header::ProtocolHeaderCodec},
};

use super::{
    engine::ConnectionEngine, ConnectionHandle, OpenError, DEFAULT_CHANNEL_MAX,
    DEFAULT_MAX_FRAME_SIZE,
};

#[cfg(feature = "tracing")]
use tracing::instrument;

pub(crate) const DEFAULT_CONTROL_CHAN_BUF: usize = 128;
pub(crate) const DEFAULT_OUTGOING_BUFFER_SIZE: usize = u16::MAX as usize;

pub(crate) mod mode {
    /// Type state for [`crate::connection::Builder`]
    #[derive(Debug)]
    pub struct ConnectorWithId {}
    /// Type state for [`crate::connection::Builder`]
    #[derive(Debug)]
    pub struct ConnectorNoId {}
}

/// Builder for [`crate::Connection`]
#[derive(Clone)]
pub struct Builder<'a, Mode, Tls> {
    /// The id of the source container
    pub container_id: String,

    /// The name of the target host
    pub hostname: Option<&'a str>,

    /// URL scheme
    pub scheme: &'a str,

    /// URL domain
    pub domain: Option<&'a str>,

    /// Proposed maximum frame size
    ///
    /// This includes the 8 bytes taken by the frame header
    pub max_frame_size: MaxFrameSize,

    /// The maximum channel number that can be used on the connection
    ///
    /// The channel-max value is the highest channel number that can be used on the connection. This
    /// value plus one is the maximum number of sessions that can be simultaneously active on the
    /// connection
    pub channel_max: ChannelMax,

    /// Idle time-out
    pub idle_time_out: Option<Milliseconds>,

    /// Locales available for outgoing text
    pub outgoing_locales: Option<Vec<IetfLanguageTag>>,

    /// Desired locales for incoming text in decreasing level of preference
    pub incoming_locales: Option<Vec<IetfLanguageTag>>,

    /// Extension capabilities the sender supports
    pub offered_capabilities: Option<Vec<Symbol>>,

    /// Extension capabilities the sender can use if the receiver supports them
    pub desired_capabilities: Option<Vec<Symbol>>,

    /// Connection properties
    pub properties: Option<Fields>,

    /// TLS connector.
    ///
    /// If `"rustls"` is enabled, this field will be `tokio_rustls::TlsConnector`.
    ///
    /// If `"native-tls"` is enabled, this field will be `tokio_native_tls::TlsConnector`.
    ///
    /// If none of the above conditions were true, this will default to unit type `()`.
    pub tls_connector: Tls,

    /// Buffer size of the underlying [`tokio::sync::mpsc::channel`] that are used by the sessions
    ///
    /// # Default
    ///
    /// ```rust, ignore
    /// u16::MAX
    /// ```
    pub buffer_size: usize,

    /// SASL profile for SASL negotiation.
    ///
    /// # Warning
    ///
    /// If username and password are supplied with the url, this field will be overriden with a
    /// PLAIN SASL profile that is interpreted from the url.
    pub sasl_profile: Option<SaslProfile>,

    /// TLS establishment
    ///
    /// This determines whether an AMQP TLS protocol header exchange will be performed prior to
    /// actual TLS handshake
    pub alt_tls_estab: bool,

    // type state marker
    marker: PhantomData<Mode>,
}

impl<'a, Tls> From<Builder<'a, mode::ConnectorWithId, Tls>> for Open {
    fn from(builder: Builder<'a, mode::ConnectorWithId, Tls>) -> Self {
        let max_frame_size = MaxFrameSize(std::cmp::max(
            MIN_MAX_FRAME_SIZE as u32,
            builder.max_frame_size.0,
        ));
        Open {
            container_id: builder.container_id,
            hostname: builder.hostname.map(Into::into),
            max_frame_size,
            channel_max: builder.channel_max,
            // To avoid spurious timeouts, the value in idle-time-out SHOULD be half the peer’s actual timeout threshold.
            idle_time_out: builder.idle_time_out.map(|v| v / 2),
            outgoing_locales: builder.outgoing_locales.map(Into::into),
            incoming_locales: builder.incoming_locales.map(Into::into),
            offered_capabilities: builder.offered_capabilities.map(Into::into),
            desired_capabilities: builder.desired_capabilities.map(Into::into),
            properties: builder.properties,
        }
    }
}

impl<'a, Mode: std::fmt::Debug> std::fmt::Debug for Builder<'a, Mode, ()> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder")
            .field("container_id", &self.container_id)
            .field("hostname", &self.hostname)
            .field("scheme", &self.scheme)
            .field("domain", &self.domain)
            .field("max_frame_size", &self.max_frame_size)
            .field("channel_max", &self.channel_max)
            .field("idle_time_out", &self.idle_time_out)
            .field("outgoing_locales", &self.outgoing_locales)
            .field("incoming_locales", &self.incoming_locales)
            .field("offered_capabilities", &self.offered_capabilities)
            .field("desired_capabilities", &self.desired_capabilities)
            .field("properties", &self.properties)
            .field("tls_connector", &"()")
            .field("buffer_size", &self.buffer_size)
            .field("sasl_profile", &self.sasl_profile)
            .field("marker", &self.marker)
            .finish()
    }
}

#[cfg(feature = "rustls")]
impl<'a, Mode: std::fmt::Debug> std::fmt::Debug for Builder<'a, Mode, tokio_rustls::TlsConnector> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder")
            .field("container_id", &self.container_id)
            .field("hostname", &self.hostname)
            .field("scheme", &self.scheme)
            .field("domain", &self.domain)
            .field("max_frame_size", &self.max_frame_size)
            .field("channel_max", &self.channel_max)
            .field("idle_time_out", &self.idle_time_out)
            .field("outgoing_locales", &self.outgoing_locales)
            .field("incoming_locales", &self.incoming_locales)
            .field("offered_capabilities", &self.offered_capabilities)
            .field("desired_capabilities", &self.desired_capabilities)
            .field("properties", &self.properties)
            .field("tls_connector", &"tokio_rustls::TlsConnector")
            .field("buffer_size", &self.buffer_size)
            .field("sasl_profile", &self.sasl_profile)
            .field("marker", &self.marker)
            .finish()
    }
}

#[cfg(feature = "native-tls")]
impl<'a, Mode: std::fmt::Debug> std::fmt::Debug
    for Builder<'a, Mode, tokio_native_tls::TlsConnector>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder")
            .field("container_id", &self.container_id)
            .field("hostname", &self.hostname)
            .field("scheme", &self.scheme)
            .field("domain", &self.domain)
            .field("max_frame_size", &self.max_frame_size)
            .field("channel_max", &self.channel_max)
            .field("idle_time_out", &self.idle_time_out)
            .field("outgoing_locales", &self.outgoing_locales)
            .field("incoming_locales", &self.incoming_locales)
            .field("offered_capabilities", &self.offered_capabilities)
            .field("desired_capabilities", &self.desired_capabilities)
            .field("properties", &self.properties)
            .field("tls_connector", &"tokio_native_tls::TlsConnector")
            .field("buffer_size", &self.buffer_size)
            .field("sasl_profile", &self.sasl_profile)
            .field("marker", &self.marker)
            .finish()
    }
}

impl<'a, Mode> Default for Builder<'a, Mode, ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Mode> Builder<'a, Mode, ()> {
    /// Creates a new builder for [`crate::Connection`]
    pub fn new() -> Self {
        Self {
            container_id: String::new(),
            hostname: None,
            scheme: "amqp", // Assume non-TLS by default
            domain: None,
            // set to 512 before Open frame is sent
            max_frame_size: MaxFrameSize(DEFAULT_MAX_FRAME_SIZE),
            channel_max: ChannelMax(DEFAULT_CHANNEL_MAX),
            idle_time_out: None,
            outgoing_locales: None,
            incoming_locales: None,
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,

            tls_connector: (),

            buffer_size: DEFAULT_OUTGOING_BUFFER_SIZE,
            sasl_profile: None,
            alt_tls_estab: false,

            marker: PhantomData,
        }
    }
}

impl<'a, Tls> Builder<'a, mode::ConnectorNoId, Tls> {
    /// The id of the source container
    pub fn container_id(self, id: impl Into<String>) -> Builder<'a, mode::ConnectorWithId, Tls> {
        // In Rust, it’s more common to pass slices as arguments
        // rather than vectors when you just want to provide read access.
        // The same goes for String and &str.
        Builder {
            container_id: id.into(),
            hostname: self.hostname,
            scheme: self.scheme,
            domain: self.domain,
            // set to 512 before Open frame is sent
            max_frame_size: self.max_frame_size,
            channel_max: self.channel_max,
            idle_time_out: self.idle_time_out,
            outgoing_locales: self.outgoing_locales,
            incoming_locales: self.incoming_locales,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            properties: self.properties,

            tls_connector: self.tls_connector,

            buffer_size: self.buffer_size,
            sasl_profile: self.sasl_profile,
            alt_tls_estab: self.alt_tls_estab,

            marker: PhantomData,
        }
    }
}

impl<'a, Mode, Tls> Builder<'a, Mode, Tls> {
    /// Alias for [`rustls_connector`](#method.rustls_connector) if only `"rustls"` is enabled
    #[cfg_attr(docsrs, doc(cfg(all(feature = "rustls", not(feature = "native-tls")))))]
    #[cfg(any(docsrs, all(feature = "rustls", not(feature = "native-tls"))))]
    pub fn tls_connector(
        self,
        tls_connector: tokio_rustls::TlsConnector,
    ) -> Builder<'a, Mode, tokio_rustls::TlsConnector> {
        self.rustls_connector(tls_connector)
    }

    /// Set the TLS connector with `tokio-rustls`
    ///
    /// If only one of `"rustls"` or `"native-tls"` is enabled, a convenience alias function `tls_connector()` is provided.
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    #[cfg(all(feature = "rustls"))]
    pub fn rustls_connector(
        self,
        tls_connector: tokio_rustls::TlsConnector,
    ) -> Builder<'a, Mode, tokio_rustls::TlsConnector> {
        // In Rust, it’s more common to pass slices as arguments
        // rather than vectors when you just want to provide read access.
        // The same goes for String and &str.
        Builder {
            container_id: self.container_id,
            hostname: self.hostname,
            scheme: self.scheme,
            domain: self.domain,
            // set to 512 before Open frame is sent
            max_frame_size: self.max_frame_size,
            channel_max: self.channel_max,
            idle_time_out: self.idle_time_out,
            outgoing_locales: self.outgoing_locales,
            incoming_locales: self.incoming_locales,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            properties: self.properties,

            tls_connector,

            buffer_size: self.buffer_size,
            sasl_profile: self.sasl_profile,
            alt_tls_estab: self.alt_tls_estab,

            marker: PhantomData,
        }
    }

    /// Alias for [`native_tls_connector`](#method.native_tls_connector) if only `"native-tls"` is
    /// enabled.
    #[cfg_attr(docsrs, doc(cfg(all(feature = "native-tls", not(feature = "rustls")))))]
    #[cfg(any(docsrs, all(feature = "native-tls", not(feature = "rustls"))))]
    pub fn tls_connector(
        self,
        tls_connector: tokio_native_tls::TlsConnector,
    ) -> Builder<'a, Mode, tokio_native_tls::TlsConnector> {
        self.native_tls_connector(tls_connector)
    }

    /// Set the TLS connector with `tokio-native-tls`
    ///
    /// If only one of `"rustls"` or `"native-tls"` is enabled, a convenience alias function `tls_connector()` is provided.
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    #[cfg(feature = "native-tls")]
    pub fn native_tls_connector(
        self,
        tls_connector: tokio_native_tls::TlsConnector,
    ) -> Builder<'a, Mode, tokio_native_tls::TlsConnector> {
        // In Rust, it’s more common to pass slices as arguments
        // rather than vectors when you just want to provide read access.
        // The same goes for String and &str.
        Builder {
            container_id: self.container_id,
            hostname: self.hostname,
            scheme: self.scheme,
            domain: self.domain,
            // set to 512 before Open frame is sent
            max_frame_size: self.max_frame_size,
            channel_max: self.channel_max,
            idle_time_out: self.idle_time_out,
            outgoing_locales: self.outgoing_locales,
            incoming_locales: self.incoming_locales,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            properties: self.properties,

            tls_connector,

            buffer_size: self.buffer_size,
            sasl_profile: self.sasl_profile,
            alt_tls_estab: self.alt_tls_estab,

            marker: PhantomData,
        }
    }
}

impl<'a, Mode, Tls> Builder<'a, Mode, Tls> {
    /// The name of the target host
    pub fn hostname(mut self, hostname: impl Into<Option<&'a str>>) -> Self {
        self.hostname = hostname.into();
        self
    }

    /// URL scheme
    pub fn scheme(mut self, scheme: &'a str) -> Self {
        self.scheme = scheme;
        self
    }

    /// URL domain
    pub fn domain(mut self, domain: impl Into<Option<&'a str>>) -> Self {
        self.domain = domain.into();
        self
    }

    /// Proposed maximum frame size
    ///
    /// This includes the 8 bytes taken by the frame header
    pub fn max_frame_size(mut self, max_frame_size: impl Into<MaxFrameSize>) -> Self {
        self.max_frame_size = max_frame_size.into();
        self
    }

    /// The maximum channel number that can be used on the connection
    ///
    /// The channel-max value is the highest channel number that can be used on the connection. This
    /// value plus one is the maximum number of sessions that can be simultaneously active on the
    /// connection
    pub fn channel_max(mut self, channel_max: impl Into<ChannelMax>) -> Self {
        self.channel_max = channel_max.into();
        self
    }

    /// The maximum number of session that can be established on this connection.
    ///
    /// This will modify the `channel-max` field. The `channel-max` plus one is the maximum
    /// number of sessions taht can be simultaenously active on the connection
    pub fn session_max(mut self, session_max: impl Into<ChannelMax>) -> Self {
        let mut channel_max = session_max.into();
        channel_max.0 -= 1;
        self.channel_max = channel_max;
        self
    }

    /// Idle time-out
    pub fn idle_time_out(mut self, idle_time_out: impl Into<Milliseconds>) -> Self {
        self.idle_time_out = Some(idle_time_out.into());
        self
    }

    /// Add one locales available for outgoing text
    pub fn add_outgoing_locales(mut self, locale: impl Into<IetfLanguageTag>) -> Self {
        match &mut self.outgoing_locales {
            Some(locales) => locales.push(locale.into()),
            None => self.outgoing_locales = Some(vec![locale.into()]),
        }
        self
    }

    /// Set the locales available for outgoing text
    pub fn set_outgoing_locales(mut self, locales: Vec<IetfLanguageTag>) -> Self {
        self.outgoing_locales = Some(locales);
        self
    }

    /// Add one desired locales for incoming text in decreasing level of preference
    pub fn add_incoming_locales(mut self, locale: impl Into<IetfLanguageTag>) -> Self {
        match &mut self.incoming_locales {
            Some(locales) => locales.push(locale.into()),
            None => self.incoming_locales = Some(vec![locale.into()]),
        }
        self
    }

    /// Set the desired locales for incoming text in decreasing level of preference
    pub fn set_incoming_locales(mut self, locales: Vec<IetfLanguageTag>) -> Self {
        self.incoming_locales = Some(locales);
        self
    }

    /// Add one extension capabilities the sender supports
    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender supports
    pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.offered_capabilities = Some(capabilities);
        self
    }

    /// Add one extension capabilities the sender can use if the receiver supports them
    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender can use if the receiver supports them
    pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.desired_capabilities = Some(capabilities);
        self
    }

    /// Connection properties
    pub fn properties(mut self, properties: Fields) -> Self {
        self.properties = Some(properties);
        self
    }

    /// Buffer size of the underlying [`tokio::sync::mpsc::channel`] that are used by the sessions
    pub fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }

    /// SASL profile for SASL negotiation.
    ///
    /// # Warning
    ///
    /// If username and password are supplied with the url, this field will be overriden with a
    /// PLAIN SASL profile that is interpreted from the url.
    pub fn sasl_profile(mut self, profile: impl Into<SaslProfile>) -> Self {
        self.sasl_profile = Some(profile.into());
        self
    }

    /// Set the alternative tls_establishment
    ///
    /// Please see part 5.2.1 of the core spec
    pub fn alt_tls_establishment(mut self, value: bool) -> Self {
        self.alt_tls_estab = value;
        self
    }
}

impl<'a, Tls> Builder<'a, mode::ConnectorWithId, Tls> {
    /// Performs SASL negotiation
    #[cfg_attr(feature = "tracing", instrument(skip_all, fields(hostname = ?self.hostname)))]
    pub async fn negotiate_sasl<Io>(
        &mut self,
        transport: &mut Transport<Io, sasl::Frame>,
        // hostname: Option<&str>,
        mut profile: SaslProfile,
    ) -> Result<(), NegotiationError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        // TODO: timeout?
        while let Some(frame) = transport.next().await {
            let frame = frame?;

            #[cfg(feature = "tracing")]
            tracing::trace!(received = ?frame);

            #[cfg(feature = "log")]
            log::trace!("received = {:?}", frame);

            match profile.on_frame(frame, self.hostname)? {
                Negotiation::Init(init) => {
                    let frame = sasl::Frame::Init(init);
                    #[cfg(feature = "tracing")]
                    tracing::trace!(sending = ?frame);
                    #[cfg(feature = "log")]
                    log::trace!("sending = {:?}", frame);
                    transport.send(frame).await?
                }
                Negotiation::Response(response) => {
                    let frame = sasl::Frame::Response(response);
                    #[cfg(feature = "tracing")]
                    tracing::trace!(sending = ?frame);
                    #[cfg(feature = "log")]
                    log::trace!("sending = {:?}", frame);
                    transport.send(frame).await?
                }
                Negotiation::Outcome(outcome) => match outcome.code {
                    SaslCode::Ok => return Ok(()),
                    code => {
                        return Err(NegotiationError::SaslError {
                            code,
                            additional_data: outcome.additional_data,
                        })
                    }
                },
            }
        }
        Err(NegotiationError::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Expecting SASL negotiation",
        )))
    }

    async fn connect_with_stream<Io>(
        mut self,
        stream: Io,
    ) -> Result<ConnectionHandle<()>, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        match self.sasl_profile.take() {
            Some(profile) => {
                let (reader, writer) = tokio::io::split(stream);
                let framed_write = FramedWrite::new(writer, ProtocolHeaderCodec::new());
                let framed_read = FramedRead::new(reader, ProtocolHeaderCodec::new());
                let mut transport =
                    Transport::negotiate_sasl_header(framed_write, framed_read).await?;
                self.negotiate_sasl(&mut transport, profile).await?;

                // NOTE: LengthDelimitedCodec itself doesn't seem to carry any buffer, so
                // it should be fine to simply drop it.
                let (framed_write, framed_read) = transport.into_framed_codec();
                let framed_write = framed_write.map_encoder(|_| ProtocolHeaderCodec::new());
                let framed_read = framed_read.map_decoder(|_| ProtocolHeaderCodec::new());

                // Then perform AMQP negotiation
                self.connect_amqp_with_framed(framed_write, framed_read)
                    .await
            }
            None => self.connect_amqp_with_stream(stream).await,
        }
    }

    async fn connect_amqp_with_framed<Io>(
        self,
        framed_write: FramedWrite<WriteHalf<Io>, ProtocolHeaderCodec>,
        framed_read: FramedRead<ReadHalf<Io>, ProtocolHeaderCodec>,
    ) -> Result<ConnectionHandle<()>, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        // Exchange AMQP headers
        let mut local_state = ConnectionState::Start;
        let idle_timeout = self
            .idle_time_out
            .map(|millis| Duration::from_millis(millis as u64));
        let buffer_size = self.buffer_size;
        let transport = Transport::negotiate_amqp_header(
            framed_write,
            framed_read,
            &mut local_state,
            idle_timeout,
        )
        .await?;

        let local_open = Open::from(self);

        // Create channels
        let (control_tx, control_rx) = mpsc::channel(DEFAULT_CONTROL_CHAN_BUF);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(buffer_size);
        let connection = Connection::new(local_state, local_open);

        let engine = ConnectionEngine::open(transport, connection, control_rx, outgoing_rx).await?;
        let handle = engine.spawn();

        let connection_handle = ConnectionHandle {
            control: control_tx,
            handle,
            outgoing: outgoing_tx, // session_control: session_control_tx
            session_listener: (),
        };

        Ok(connection_handle)
    }

    async fn connect_amqp_with_stream<Io>(
        self,
        stream: Io,
    ) -> Result<ConnectionHandle<()>, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        let (reader, writer) = tokio::io::split(stream);
        let framed_write = FramedWrite::new(writer, ProtocolHeaderCodec::new());
        let framed_read = FramedRead::new(reader, ProtocolHeaderCodec::new());
        self.connect_amqp_with_framed(framed_write, framed_read)
            .await
    }
}

impl<'a> Builder<'a, mode::ConnectorWithId, ()> {
    /// Open a [`crate::Connection`] with an url
    ///
    /// # Raw AMQP connection
    ///
    /// ```rust
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .open("amqp://localhost:5672")
    ///     .await.unwrap();
    /// ```
    ///
    /// # TLS
    ///
    /// TLS is not supported unless one and only one of the following feature must be enabled
    ///
    /// 1. "rustls"
    /// 2. "native-tls"
    ///
    /// If no custom `TlsConnector` is supplied, the following default connector will be used.
    ///
    /// ## Default TLS connector with `"rustls"` enabled
    ///
    /// ```rust,ignore
    /// let mut root_cert_store = RootCertStore::empty();
    /// root_cert_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(
    ///     |ta| {
    ///         OwnedTrustAnchor::from_subject_spki_name_constraints(
    ///             ta.subject,
    ///             ta.spki,
    ///             ta.name_constraints,
    ///         )
    ///     },
    /// ));
    /// let config = ClientConfig::builder()
    ///     .with_safe_defaults()
    ///     .with_root_certificates(root_cert_store)
    ///     .with_no_client_auth();
    /// let connector = TlsConnector::from(Arc::new(config));
    /// ```
    ///
    /// ## Default TLS connector with `"native-tls"` enabled
    ///
    /// ```rust,ignore
    /// let connector = native_tls::TlsConnector::new().unwrap();
    /// let connector = tokio_native_tls::TlsConnector::from(connector);
    /// ```
    ///
    /// # SASL
    ///
    /// ```rust, ignore
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .open("amqp://guest:guest@localhost:5672")
    ///     .await.unwrap();
    ///
    /// // Or you can supply the SASL profile to the builder
    /// let profile = SaslProfile::Plain {
    ///     username: "guest".to_string(),
    ///     password: "guest".to_string()
    /// };
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .sasl_profile(profile)
    ///     .open("amqp://localhost:5672")
    ///     .await.unwrap();
    /// ```
    ///
    pub async fn open(
        mut self,
        url: impl TryInto<Url, Error = impl Into<OpenError>>,
    ) -> Result<ConnectionHandle<()>, OpenError> {
        let url = url.try_into().map_err(Into::into)?;

        // Url info will override the builder fields
        // only override if value exists
        self.scheme = url.scheme();
        if let Some(hostname) = url.host_str() {
            self.hostname = Some(hostname);
        }
        if let Some(domain) = url.domain() {
            self.domain = Some(domain);
        }
        if let Ok(profile) = SaslProfile::try_from(&url) {
            self.sasl_profile = Some(profile);
        }

        let addr = url.socket_addrs(|| default_port(url.scheme()))?;
        let stream = TcpStream::connect(&*addr).await?; // std::io::Error

        self.open_with_stream(stream).await
    }

    /// Open with an IO that implements `AsyncRead` and `AsyncWrite`.
    ///
    /// The stream will be wrapped in `BufReader` and `BufWriter` so it is not necessary
    /// to wrap the stream in buffer.
    ///
    /// # TLS
    ///
    /// If the `scheme` field is `"amqps"`, the builder will attempt to start with
    /// exchanging TLS protocol header.
    ///
    /// # Alternative TLS establishment
    ///
    /// This can be used for alternative connection establishment over a TLS stream
    /// **without** exchanging the TLS protocol header (['A', 'M', 'Q', 'P', 2, 1, 0, 0]).
    ///
    /// An example of establishing connection on a `tokio_native_tls::TlsStream` is shown below.
    /// The `tls_stream` can be replaced with a `tokio_rustls::client::TlsStream`.
    ///
    /// ```rust,ignore
    /// let addr = "localhost:5671";
    /// let domain = "localhost";
    /// let stream = TcpStream::connect(addr).await.unwrap();
    /// let connector = native_tls::TlsConnector::new();
    /// let connector = tokio_native_tls::TlsConnector::from(connector);
    /// let tls_stream = connector.connect(domain, stream).await.unwrap();
    ///
    /// let mut connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .scheme("amqp")
    ///     .sasl_profile(SaslProfile::Plain {
    ///         username: "guest".into(),
    ///         password: "guest".into()
    ///     })
    ///     .open_with_stream(tls_stream)
    ///     .await
    ///     .unwrap();
    /// ```
    #[allow(unreachable_code)]
    pub async fn open_with_stream<Io>(self, stream: Io) -> Result<ConnectionHandle<()>, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        match self.scheme {
            "amqp" => self.connect_with_stream(stream).await,
            "amqps" => {
                #[cfg(all(feature = "rustls", not(feature = "native-tls")))]
                {
                    let domain = self.domain.ok_or(OpenError::InvalidDomain)?;
                    return self.connect_tls_with_rustls_default(stream, domain).await;
                }

                #[cfg(all(feature = "native-tls", not(feature = "rustls")))]
                {
                    let domain = self.domain.ok_or_else(|| OpenError::InvalidDomain)?;
                    return self
                        .connect_tls_with_native_tls_default(stream, domain)
                        .await;
                }

                Err(OpenError::TlsConnectorNotFound)
            }
            _ => Err(OpenError::InvalidScheme),
        }
    }

    #[cfg(all(feature = "rustls", not(feature = "native-tls")))]
    async fn connect_tls_with_rustls_default<Io>(
        self,
        stream: Io,
        domain: &str,
    ) -> Result<ConnectionHandle<()>, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        use librustls::{ClientConfig, OwnedTrustAnchor, RootCertStore};
        use std::sync::Arc;
        use tokio_rustls::TlsConnector;

        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(
            |ta| {
                OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )
            },
        ));
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(config));
        let tls_stream =
            Transport::connect_tls_with_rustls(stream, domain, &connector, self.alt_tls_estab)
                .await?;
        self.connect_with_stream(tls_stream).await
    }

    #[cfg(all(feature = "native-tls", not(feature = "rustls")))]
    async fn connect_tls_with_native_tls_default<Io>(
        self,
        stream: Io,
        domain: &str,
    ) -> Result<ConnectionHandle<()>, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        let connector = libnative_tls::TlsConnector::new()
            .map_err(|e| OpenError::Io(io::Error::new(io::ErrorKind::Other, format!("{:?}", e))))?;
        let connector = tokio_native_tls::TlsConnector::from(connector);
        let tls_stream =
            Transport::connect_tls_with_native_tls(stream, domain, &connector, self.alt_tls_estab)
                .await?;
        self.connect_with_stream(tls_stream).await
    }
}

#[cfg(all(feature = "rustls"))]
impl<'a> Builder<'a, mode::ConnectorWithId, tokio_rustls::TlsConnector> {
    /// Open a [`crate::Connection`] with an url
    ///
    /// # Raw AMQP connection
    ///
    /// ```rust
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .open("amqp://localhost:5672")
    ///     .await.unwrap();
    /// ```
    ///
    /// # TLS
    ///
    /// TLS is supported with either `tokio-rustls` or `tokio-native-tls` by choosing the
    /// corresponding feature flag.
    ///
    /// ## TLS with feature `"rustls"` enabled
    ///
    /// ```rust, ignore
    /// let config = rustls::ClientConfig::builder()
    ///     .with_safe_defaults()
    ///     .with_root_certificates(root_cert_store)
    ///     .with_no_client_auth(); // i guess this was previously the default?
    /// let connector = TlsConnector::from(Arc::new(config));
    ///
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .tls_connector(connector)
    ///     .open("amqps://guest:guest@localhost:5671")
    ///     .await.unwrap();
    /// ```
    ///
    /// ## TLS with feature `"native-tls"` enabled
    ///
    /// ```rust,ignore
    /// let cx = native_tls::TlsConnector::new();
    /// let connector = tokio_native_tls::TlsConnector::from(cx);
    ///
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .tls_connector(connector)
    ///     .open("amqps://guest:guest@localhost:5671")
    ///     .await.unwrap();
    /// ```
    ///
    /// # SASL
    ///
    /// ```rust, ignore
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .open("amqp://guest:guest@localhost:5672")
    ///     .await.unwrap();
    ///
    /// // Or you can supply the SASL profile to the builder
    /// let profile = SaslProfile::Plain {
    ///     username: "guest".to_string(),
    ///     password: "guest".to_string()
    /// };
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .sasl_profile(profile)
    ///     .open("amqp://localhost:5672")
    ///     .await.unwrap();
    /// ```
    ///
    pub async fn open(
        mut self,
        url: impl TryInto<Url, Error = impl Into<OpenError>>,
    ) -> Result<ConnectionHandle<()>, OpenError> {
        let url = url.try_into().map_err(Into::into)?;

        // Url info will override the builder fields
        // only override if value exists
        self.scheme = url.scheme();
        if let Some(hostname) = url.host_str() {
            self.hostname = Some(hostname);
        }
        if let Some(domain) = url.domain() {
            self.domain = Some(domain);
        }
        if let Ok(profile) = SaslProfile::try_from(&url) {
            self.sasl_profile = Some(profile);
        }

        let addr = url.socket_addrs(|| default_port(url.scheme()))?;
        let stream = TcpStream::connect(&*addr).await?; // std::io::Error

        self.open_with_stream(stream).await
    }

    /// Open with an IO that implements `AsyncRead` and `AsyncWrite`
    ///
    /// # TLS
    ///
    /// If the `scheme` field is `"amqps"`, the builder will attempt to start with
    /// exchanging TLS protocol header and establish TLS stream using the user-supplied
    /// `tokio_rustls::TlsConnector`.
    pub async fn open_with_stream<Io>(self, stream: Io) -> Result<ConnectionHandle<()>, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        match self.scheme {
            "amqp" => self.connect_with_stream(stream).await,
            "amqps" => {
                let domain = self.domain.ok_or(OpenError::InvalidDomain)?;
                let tls_stream = Transport::connect_tls_with_rustls(
                    stream,
                    domain,
                    &self.tls_connector,
                    self.alt_tls_estab,
                )
                .await?;
                self.connect_with_stream(tls_stream).await
            }
            _ => Err(OpenError::InvalidScheme),
        }
    }
}

#[cfg(all(feature = "native-tls"))]
impl<'a> Builder<'a, mode::ConnectorWithId, tokio_native_tls::TlsConnector> {
    /// Open a [`crate::Connection`] with an url
    ///
    /// # Raw AMQP connection
    ///
    /// ```rust
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .open("amqp://localhost:5672")
    ///     .await.unwrap();
    /// ```
    ///
    /// # TLS
    ///
    /// TLS is supported with either `tokio-rustls` or `tokio-native-tls` by choosing the
    /// corresponding feature flag.
    ///
    /// ## TLS with feature `"rustls"` enabled
    ///
    /// ```rust, ignore
    /// let config = rustls::ClientConfig::builder()
    ///     .with_safe_defaults()
    ///     .with_root_certificates(root_cert_store)
    ///     .with_no_client_auth(); // i guess this was previously the default?
    /// let connector = TlsConnector::from(Arc::new(config));
    ///
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .tls_connector(connector)
    ///     .open("amqps://guest:guest@localhost:5671")
    ///     .await.unwrap();
    /// ```
    ///
    /// ## TLS with feature `"native-tls"` enabled
    ///
    /// ```rust,ignore
    /// let cx = native_tls::TlsConnector::new();
    /// let connector = tokio_native_tls::TlsConnector::from(cx);
    ///
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .tls_connector(connector)
    ///     .open("amqps://guest:guest@localhost:5671")
    ///     .await.unwrap();
    /// ```
    ///
    /// # SASL
    ///
    /// ```rust, ignore
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .open("amqp://guest:guest@localhost:5672")
    ///     .await.unwrap();
    ///
    /// // Or you can supply the SASL profile to the builder
    /// let profile = SaslProfile::Plain {
    ///     username: "guest".to_string(),
    ///     password: "guest".to_string()
    /// };
    /// let connection = Connection::builder()
    ///     .container_id("connection-1")
    ///     .sasl_profile(profile)
    ///     .open("amqp://localhost:5672")
    ///     .await.unwrap();
    /// ```
    ///
    pub async fn open(
        mut self,
        url: impl TryInto<Url, Error = impl Into<OpenError>>,
    ) -> Result<ConnectionHandle<()>, OpenError> {
        let url = url.try_into().map_err(Into::into)?;
        
        // Url info will override the builder fields
        // only override if value exists
        self.scheme = url.scheme();
        if let Some(hostname) = url.host_str() {
            self.hostname = Some(hostname);
        }
        if let Some(domain) = url.domain() {
            self.domain = Some(domain);
        }
        if let Ok(profile) = SaslProfile::try_from(&url) {
            self.sasl_profile = Some(profile);
        }

        let addr = url.socket_addrs(|| default_port(url.scheme()))?;
        let stream = TcpStream::connect(&*addr).await?; // std::io::Error

        self.open_with_stream(stream).await
    }

    /// Open with an IO that implements `AsyncRead` and `AsyncWrite`
    ///
    /// # TLS
    ///
    /// If the `scheme` field is `"amqps"`, the builder will attempt to start with
    /// exchanging TLS protocol header and establish TLS stream using the user-supplied
    /// `tokio_rustls::TlsConnector`.
    pub async fn open_with_stream<Io>(self, stream: Io) -> Result<ConnectionHandle<()>, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        match self.scheme {
            "amqp" => self.connect_with_stream(stream).await,
            "amqps" => {
                let domain = self.domain.ok_or(OpenError::InvalidDomain)?;
                let tls_stream = Transport::connect_tls_with_native_tls(
                    stream,
                    domain,
                    &self.tls_connector,
                    self.alt_tls_estab,
                )
                .await?;
                self.connect_with_stream(tls_stream).await
            }
            _ => Err(OpenError::InvalidScheme),
        }
    }
}

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "amqp" => Some(fe2o3_amqp_types::definitions::PORT),
        "amqps" => Some(fe2o3_amqp_types::definitions::SECURE_PORT),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    #[test]
    fn test_url_name_resolution() {
        let url: Url = "amqp://example.net/".try_into().unwrap();
        assert_eq!(url.port(), Some(5672));
        let _addrs = url.socket_addrs(|| Some(5672)).unwrap();
    }
}
