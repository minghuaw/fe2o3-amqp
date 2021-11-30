use std::{convert::TryInto, marker::PhantomData, time::Duration};

use fe2o3_amqp_types::{
    definitions::{Fields, IetfLanguageTag, Milliseconds, MIN_MAX_FRAME_SIZE},
    performatives::{ChannelMax, MaxFrameSize, Open},
};
use serde_amqp::primitives::Symbol;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use url::Url;

use crate::{
    connection::{Connection, ConnectionState},
    error::EngineError,
    transport::protocol_header::ProtocolHeader,
    transport::Transport,
};

use super::engine::ConnectionEngine;
use super::ConnectionHandle;

pub(crate) const DEFAULT_CONTROL_CHAN_BUF: usize = 128;
pub const DEFAULT_OUTGOING_BUFFER_SIZE: usize = u16::MAX as usize;

pub struct WithoutContainerId {}
pub struct WithContainerId {}

/// Connection builder
pub struct Builder<Mode> {
    pub container_id: String,
    pub hostname: Option<String>,
    pub max_frame_size: MaxFrameSize,
    pub channel_max: ChannelMax,
    pub idle_time_out: Option<Milliseconds>,
    pub outgoing_locales: Option<Vec<IetfLanguageTag>>,
    pub incoming_locales: Option<Vec<IetfLanguageTag>>,
    pub offered_capabilities: Option<Vec<Symbol>>,
    pub desired_capabilities: Option<Vec<Symbol>>,
    pub properties: Option<Fields>,

    pub buffer_size: usize,
    // type state marker
    marker: PhantomData<Mode>,
}

impl Builder<WithoutContainerId> {
    pub fn new() -> Self {
        Self {
            container_id: String::new(),
            hostname: None,
            // set to 512 before Open frame is sent
            max_frame_size: MaxFrameSize(512),
            channel_max: ChannelMax::default(),
            idle_time_out: None,
            outgoing_locales: None,
            incoming_locales: None,
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,

            buffer_size: DEFAULT_OUTGOING_BUFFER_SIZE,
            marker: PhantomData,
        }
    }
}

impl<Mode> Builder<Mode> {
    // In Rust, it’s more common to pass slices as arguments
    // rather than vectors when you just want to provide read access.
    // The same goes for String and &str.
    pub fn container_id(self, id: impl Into<String>) -> Builder<WithContainerId> {
        Builder::<WithContainerId> {
            container_id: id.into(),
            hostname: None,
            // set to 512 before Open frame is sent
            max_frame_size: MaxFrameSize(512),
            channel_max: ChannelMax::default(),
            idle_time_out: None,
            outgoing_locales: None,
            incoming_locales: None,
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,

            buffer_size: self.buffer_size,
            marker: PhantomData,
        }
    }
}

impl<Mode> Builder<Mode> {
    pub fn hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = Some(hostname.into());
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

impl Builder<WithContainerId> {
    pub async fn open_with_stream<Io>(self, mut stream: Io) -> Result<ConnectionHandle, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        use tokio::sync::mpsc;

        // exchange header
        let mut local_state = ConnectionState::Start;
        let _remote_header =
            Transport::negotiate(&mut stream, &mut local_state, ProtocolHeader::amqp()).await?;
        let idle_timeout = self
            .idle_time_out
            .map(|millis| Duration::from_millis(millis as u64));
        let transport = Transport::bind(stream, self.max_frame_size.0 as usize, idle_timeout);
        println!(">>> Debug: Header exchanged");

        // spawn Connection Mux
        let local_open = Open {
            container_id: self.container_id,
            hostname: self.hostname,
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

    pub async fn open(
        self,
        url: impl TryInto<Url, Error = url::ParseError>,
    ) -> Result<ConnectionHandle, EngineError> {
        let url: Url = url.try_into()?;

        let addr = url.socket_addrs(|| match url.scheme() {
            // check scheme
            "amqp" => Some(fe2o3_amqp_types::definitions::PORT),
            "amqps" => todo!(),
            _ => None,
        })?;
        let stream = TcpStream::connect(&*addr).await?;
        println!("TcpStream connected");
        self.open_with_stream(stream).await
    }

    pub async fn pipelined_open_with_stream<Io>(
        self,
        mut _stream: Io,
    ) -> Result<ConnectionHandle, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        todo!()
    }

    pub async fn pipelined_open(
        &self,
        _url: impl TryInto<Url>,
    ) -> Result<ConnectionHandle, EngineError> {
        todo!()
    }
}
