use std::{convert::TryInto, marker::PhantomData, time::Duration};

use fe2o3_amqp_types::{
    definitions::{Fields, IetfLanguageTag, Milliseconds, MIN_MAX_FRAME_SIZE},
    performatives::{ChannelMax, MaxFrameSize, Open},
};
use rustls::ClientConfig;
use serde_amqp::primitives::Symbol;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use url::Url;

use crate::{
    connection::{Connection, ConnectionState},
    frames::amqp,
    sasl_profile::SaslProfile,
    transport::protocol_header::ProtocolHeader,
    transport::Transport,
};

use super::{engine::ConnectionEngine, ConnectionHandle, OpenError};

pub(crate) const DEFAULT_CONTROL_CHAN_BUF: usize = 128;
pub const DEFAULT_OUTGOING_BUFFER_SIZE: usize = u16::MAX as usize;

pub struct WithoutContainerId {}
pub struct WithContainerId {}

/// Connection builder
pub struct Builder<'a, Mode> {
    pub container_id: String,
    pub hostname: Option<&'a str>,
    pub scheme: &'a str,
    pub domain: Option<&'a str>,
    pub max_frame_size: MaxFrameSize,
    pub channel_max: ChannelMax,
    pub idle_time_out: Option<Milliseconds>,
    pub outgoing_locales: Option<Vec<IetfLanguageTag>>,
    pub incoming_locales: Option<Vec<IetfLanguageTag>>,
    pub offered_capabilities: Option<Vec<Symbol>>,
    pub desired_capabilities: Option<Vec<Symbol>>,
    pub properties: Option<Fields>,

    pub client_config: Option<ClientConfig>,

    pub buffer_size: usize,
    pub sasl_profile: Option<SaslProfile>,

    // type state marker
    marker: PhantomData<Mode>,
}

impl<'a> Builder<'a, WithoutContainerId> {
    pub fn new() -> Self {
        Self {
            container_id: String::new(),
            hostname: None,
            scheme: "amqp".into(), // Assume non-TLS by default
            domain: None,
            // set to 512 before Open frame is sent
            max_frame_size: MaxFrameSize(MIN_MAX_FRAME_SIZE as u32),
            channel_max: ChannelMax::default(),
            idle_time_out: None,
            outgoing_locales: None,
            incoming_locales: None,
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,

            client_config: None,

            buffer_size: DEFAULT_OUTGOING_BUFFER_SIZE,
            sasl_profile: None,

            marker: PhantomData,
        }
    }
}

impl<'a, Mode> Builder<'a, Mode> {
    // In Rust, it’s more common to pass slices as arguments
    // rather than vectors when you just want to provide read access.
    // The same goes for String and &str.
    pub fn container_id(self, id: impl Into<String>) -> Builder<'a, WithContainerId> {
        Builder::<WithContainerId> {
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

            client_config: self.client_config,

            buffer_size: self.buffer_size,
            sasl_profile: self.sasl_profile,
            marker: PhantomData,
        }
    }
}

impl<'a, Mode> Builder<'a, Mode> {
    pub fn hostname(mut self, hostname: impl Into<Option<&'a str>>) -> Self {
        self.hostname = hostname.into();
        self
    }

    pub fn scheme(mut self, scheme: impl Into<&'a str>) -> Self {
        self.scheme = scheme.into();
        self
    }

    pub fn domain(mut self, domain: impl Into<Option<&'a str>>) -> Self {
        self.domain = domain.into();
        self
    }

    pub fn max_frame_size(mut self, max_frame_size: impl Into<MaxFrameSize>) -> Self {
        let max_frame_size = max_frame_size.into();
        let max_frame_size = std::cmp::max(MIN_MAX_FRAME_SIZE as u32, max_frame_size.0);
        self.max_frame_size = MaxFrameSize::from(max_frame_size);
        self
    }

    pub fn channel_max(mut self, channel_max: impl Into<ChannelMax>) -> Self {
        self.channel_max = channel_max.into();
        self
    }

    pub fn idle_time_out(mut self, idle_time_out: impl Into<Milliseconds>) -> Self {
        self.idle_time_out = Some(idle_time_out.into());
        self
    }

    pub fn add_outgoing_locales(mut self, locale: impl Into<IetfLanguageTag>) -> Self {
        match &mut self.outgoing_locales {
            Some(locales) => locales.push(locale.into()),
            None => self.outgoing_locales = Some(vec![locale.into()]),
        }
        self
    }

    pub fn set_outgoing_locales(mut self, locales: Vec<IetfLanguageTag>) -> Self {
        self.outgoing_locales = Some(locales);
        self
    }

    pub fn add_incoming_locales(mut self, locale: impl Into<IetfLanguageTag>) -> Self {
        match &mut self.incoming_locales {
            Some(locales) => locales.push(locale.into()),
            None => self.incoming_locales = Some(vec![locale.into()]),
        }
        self
    }

    pub fn set_incoming_locales(mut self, locales: Vec<IetfLanguageTag>) -> Self {
        self.incoming_locales = Some(locales);
        self
    }

    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.offered_capabilities = Some(capabilities);
        self
    }

    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.desired_capabilities = Some(capabilities);
        self
    }

    pub fn properties(mut self, properties: Fields) -> Self {
        self.properties = Some(properties);
        self
    }

    pub fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }
}

impl<'a> Builder<'a, WithContainerId> {
    pub async fn open_with_stream<Io>(mut self, stream: Io) -> Result<ConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        match self.sasl_profile.take() {
            Some(profile) => {
                let hostname = self.hostname;
                let scheme = self.scheme;
                let domain = self.domain;
                let stream = Transport::connect_sasl(stream, hostname, profile).await?;
                self.connect_with_stream(stream, scheme, domain).await
            }
            None => {
                let scheme = self.scheme;
                let domain = self.domain;
                self.connect_with_stream(stream, scheme, domain).await
            }
        }
    }

    pub async fn open(
        mut self,
        url: impl TryInto<Url, Error = url::ParseError>,
    ) -> Result<ConnectionHandle, OpenError> {
        let url: Url = url.try_into()?;

        // Url info will override the builder fields
        self.hostname = url.host_str().map(Into::into);
        self.scheme = url.scheme().into();
        self.domain = url.domain().map(Into::into);
        self.sasl_profile = SaslProfile::try_from(&url).ok();

        let addr = url.socket_addrs(|| Some(fe2o3_amqp_types::definitions::PORT))?;
        let stream = TcpStream::connect(&*addr).await?; // std::io::Error
        self.open_with_stream(stream).await
    }

    // pub async fn pipelined_open_with_stream<Io>(
    //     self,
    //     mut _stream: Io,
    // ) -> Result<ConnectionHandle, Error>
    // where
    //     Io: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    // {
    //     todo!()
    // }

    // pub async fn pipelined_open(&self, _url: impl TryInto<Url>) -> Result<ConnectionHandle, Error> {
    //     todo!()
    // }

    async fn connect_with_stream<Io>(
        self,
        stream: Io,
        scheme: &str,
        domain: Option<&str>,
    ) -> Result<ConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        match scheme {
            "amqp" => self.connect_with_stream_inner(stream).await,
            "amqps" => {
                let domain = domain.ok_or_else(|| OpenError::InvalidDomain)?;
                let config = self.client_config.clone()
                    .ok_or_else(|| OpenError::TlsClientConfigNotFound)?;
                let tls_stream = Transport::connect_tls(stream, domain, config).await?;
                self.connect_with_stream_inner(tls_stream).await
            }
            _ => Err(OpenError::InvalidScheme),
        }
    }

    async fn connect_with_stream_inner<Io>(self, mut stream: Io) -> Result<ConnectionHandle, OpenError>
    where
        Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    {
        use tokio::sync::mpsc;

        // exchange header
        let mut local_state = ConnectionState::Start;
        let _remote_header =
            Transport::negotiate(&mut stream, &mut local_state, ProtocolHeader::amqp()).await?;
        let idle_timeout = self
            .idle_time_out
            .map(|millis| Duration::from_millis(millis as u64));
        let transport =
            Transport::<_, amqp::Frame>::bind(stream, self.max_frame_size.0 as usize, idle_timeout);

        // spawn Connection Mux
        let local_open = Open {
            container_id: self.container_id,
            hostname: self.hostname.map(Into::into),
            max_frame_size: self.max_frame_size,
            channel_max: self.channel_max,
            // To avoid spurious timeouts, the value in idle-time-out SHOULD be half the peer’s actual timeout threshold.
            idle_time_out: self.idle_time_out.map(|v| v / 2),
            outgoing_locales: self.outgoing_locales,
            incoming_locales: self.incoming_locales,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            properties: self.properties,
        };

        // create channels
        let (connection_control_tx, connection_control_rx) =
            mpsc::channel(DEFAULT_CONTROL_CHAN_BUF);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(self.buffer_size);

        let connection = Connection::new(connection_control_tx.clone(), local_state, local_open);
        let engine = ConnectionEngine::open(
            transport,
            connection,
            connection_control_rx,
            outgoing_rx,
            // session_control_rx
        )
        .await?;
        let handle = engine.spawn();

        let connection_handle = ConnectionHandle {
            control: connection_control_tx,
            handle,
            outgoing: outgoing_tx, // session_control: session_control_tx
        };

        Ok(connection_handle)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_url_name_resolution() {
        let url = url::Url::parse("amqp://example.net/").unwrap();
        let addrs = url.socket_addrs(|| Some(5671)).unwrap();
        println!("{:?}", addrs);
    }
}
