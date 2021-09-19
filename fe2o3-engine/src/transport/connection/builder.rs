use std::{convert::{TryInto}, marker::PhantomData, time::Duration};

use fe2o3_amqp::primitives::{Symbol};
use fe2o3_types::{definitions::{Fields, IetfLanguageTag, Milliseconds}, performatives::{ChannelMax, MaxFrameSize, Open}};
use tokio::{io::{AsyncRead, AsyncWrite}, net::TcpStream};
use url::Url;

use crate::{error::EngineError, transport::{Transport, connection::{ConnectionState, mux::Mux}}, transport::protocol_header::ProtocolHeader};

use super::{Connection, MIN_MAX_FRAME_SIZE, mux::{self, DEFAULT_CONNECTION_MUX_BUFFER_SIZE}};

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
    marker: PhantomData<Mode>
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

            buffer_size: DEFAULT_CONNECTION_MUX_BUFFER_SIZE,
            marker: PhantomData
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
            marker: PhantomData
        }
    }
}

impl<Mode> Builder<Mode> {
    pub fn hostname(&mut self, hostname: impl Into<String>) -> &mut Self {
        self.hostname = Some(hostname.into());
        self
    }

    pub fn max_frame_size(&mut self, max_frame_size: impl Into<MaxFrameSize>) -> &mut Self {
        let max_frame_size = max_frame_size.into();
        let max_frame_size = std::cmp::max(MIN_MAX_FRAME_SIZE, max_frame_size.0);
        self.max_frame_size = MaxFrameSize::from(max_frame_size);
        self
    }

    pub fn channel_max(&mut self, channel_max: impl Into<ChannelMax>) -> &mut Self {
        self.channel_max = channel_max.into();
        self
    }

    pub fn idle_time_out(&mut self, idle_time_out: impl Into<Milliseconds>) -> &mut Self {
        self.idle_time_out = Some(idle_time_out.into());
        self
    }

    pub fn add_outgoing_locales(&mut self, locale: impl Into<IetfLanguageTag>) -> &mut Self {
        match &mut self.outgoing_locales {
            Some(locales) => locales.push(locale.into()),
            None => {
                self.outgoing_locales = Some(vec![locale.into()])
            }
        }
        self
    }

    pub fn set_outgoing_locales(&mut self, locales: Vec<IetfLanguageTag>) -> &mut Self {
        self.outgoing_locales = Some(locales);
        self
    }

    pub fn add_incoming_locales(&mut self, locale: impl Into<IetfLanguageTag>) -> &mut Self {
        match &mut self.incoming_locales {
            Some(locales) => locales.push(locale.into()),
            None => self.incoming_locales = Some(vec![locale.into()])
        }
        self
    }

    pub fn add_offered_capabilities(&mut self, capability: impl Into<Symbol>) -> &mut Self {
        match &mut self.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.offered_capabilities = Some(vec![capability.into()])
        }
        self
    }

    pub fn set_offered_capabilities(&mut self, capabilities: Vec<Symbol>) -> &mut Self {
        self.offered_capabilities = Some(capabilities);
        self
    }

    pub fn add_desired_capabilities(&mut self, capability: impl Into<Symbol>) -> &mut Self {
        match &mut self.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.desired_capabilities = Some(vec![capability.into()])
        }
        self
    }

    pub fn set_desired_capabilities(&mut self, capabilities: Vec<Symbol>) -> &mut Self {
        self.desired_capabilities = Some(capabilities);
        self
    }

    pub fn properties(&mut self, properties: Fields) -> &mut Self {
        self.properties = Some(properties);
        self
    }

    pub fn buffer_size(&mut self, buffer_size: usize) -> &mut Self {
        self.buffer_size = buffer_size;
        self
    }
}

impl Builder<WithContainerId> {
    pub async fn open_with_stream<Io>(&self, mut stream: Io) -> Result<Connection, EngineError> 
    where 
        Io: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        // exchange header 
        let mut local_state = ConnectionState::Start;
        let _remote_header = Transport::negotiate(&mut stream, &mut local_state, ProtocolHeader::amqp()).await?;
        let idle_timeout = self.idle_time_out.map(|millis| Duration::from_millis(millis as u64)); 
        let transport = Transport::bind(stream, self.max_frame_size.0 as usize, idle_timeout);
        println!("Header exchanged");

        // spawn Connection Mux
        let local_open = Open {
            container_id: self.container_id.clone(),
            hostname: self.hostname.clone(),
            max_frame_size: self.max_frame_size.clone(),
            channel_max: self.channel_max.clone(),
            // To avoid spurious timeouts, the value in idle-time-out SHOULD be half the peer’s actual timeout threshold.
            idle_time_out: self.idle_time_out.clone().map(|v| v/2), 
            outgoing_locales: self.outgoing_locales.clone(),
            incoming_locales: self.incoming_locales.clone(),
            offered_capabilities: self.offered_capabilities.clone(),
            desired_capabilities: self.desired_capabilities.clone(),
            properties: self.properties.clone()
        };

        println!(">>> Debug: with_stream() - starting Mux");
        // open Connection
        let mux = Mux::open(
            transport, 
            local_state, 
            local_open, 
            // remote_header, 
            self.buffer_size
        ).await?;
        let connection = Connection::from(mux);
        Ok(connection)
    }

    pub async fn open(&self, url: impl TryInto<Url, Error=url::ParseError>) -> Result<Connection, EngineError> {
        let url: Url = url.try_into()?;
        
        // check scheme
        match url.scheme() {
            "amqp" => {
                // connect TcpStream
                let addr = url.socket_addrs(|| Some(fe2o3_types::definitions::PORT))?;
                let stream = TcpStream::connect(&*addr).await?;
                println!("TcpStream connected");

                self.open_with_stream(stream).await
            },
            // TLS
            "amqps" => {
                todo!()
            },
            _ => return Err(EngineError::Message("Invalid Url Scheme"))
        }
    }

    pub async fn pipelined_open_with_stream<Io>(&self, mut stream: Io) -> Result<Connection, EngineError> 
    where 
        Io: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    { 
        todo!()
    }

    pub async fn pipelined_open(&self, url: impl TryInto<Url>) -> Result<Connection, EngineError> {
        todo!()
    }
}