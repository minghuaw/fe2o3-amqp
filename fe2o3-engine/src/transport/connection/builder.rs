use std::{convert::{TryFrom, TryInto}, marker::PhantomData, sync::Arc};

use fe2o3_amqp::primitives::{Symbol};
use fe2o3_types::{definitions::{Fields, IetfLanguageTag, Milliseconds}, performatives::{ChannelMax, MaxFrameSize, Open}};
use tokio::{io::{AsyncRead, AsyncWrite}, net::TcpStream};
use url::Url;

use crate::{error::EngineError, transport::{Transport, connection::{ConnectionState, mux::Mux}}, transport::protocol_header::ProtocolHeader};

use super::{Connection, mux::{self, DEFAULT_CONNECTION_MUX_BUFFER_SIZE}};

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
    pub fn container_id(self, id: String) -> Builder<WithContainerId> {
        Builder::<WithContainerId> {
            container_id: id,
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
        self.max_frame_size = max_frame_size.into();
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
    // pub async fn create_connection<U>(self, address: U) -> Result<Connection<TcpStream>, EngineError> 
    // where 
    //     U: TryInto<Url, Error=url::ParseError>,
    // {
    //     let url: Url = address.try_into()
    //         .map_err(|err| EngineError::UrlError(err))?;

    //     match url.scheme() {
    //         "amqp" => {
    //             let addr = url.socket_addrs(|| Some(fe2o3_types::definitions::PORT))?;
    //             let mut stream = TcpStream::connect(&*addr).await?;

    //             // Negotiate and then bind
    //             let remote_header = Transport::negotiate(&mut stream, ProtocolHeader::amqp()).await?;
    //             let transport = Transport::bind(stream)?;

    //             // Send Open frame
    //             // let local_open =
    //             todo!()
    //         },
    //         "amqps" => {
    //             todo!()
    //         }
    //         _ => {
    //             return Err(EngineError::Message("Invalid Url scheme"))
    //         }
    //     }
    //     todo!()
    // }

    pub async fn open(&self, url: impl TryInto<Url, Error=url::ParseError>) -> Result<Connection, EngineError> {
        let url: Url = url.try_into()?;
        
        // check scheme
        match url.scheme() {
            "amqp" => {
                // connect TcpStream
                let addr = url.socket_addrs(|| Some(fe2o3_types::definitions::PORT))?;
                let mut stream = TcpStream::connect(&*addr).await?;
                
                // exchange header 
                let mut local_state = ConnectionState::Start;
                let remote_header = Transport::negotiate(&mut stream, &mut local_state, ProtocolHeader::amqp()).await?;
                let transport = Transport::bind(stream);

                // spawn Connection Mux
                let local_open = Open {
                    container_id: self.container_id.clone(),
                    hostname: self.hostname.clone(),
                    max_frame_size: self.max_frame_size.clone(),
                    channel_max: self.channel_max.clone(),
                    idle_time_out: self.idle_time_out.clone(),
                    outgoing_locales: self.outgoing_locales.clone(),
                    incoming_locales: self.incoming_locales.clone(),
                    offered_capabilities: self.offered_capabilities.clone(),
                    desired_capabilities: self.desired_capabilities.clone(),
                    properties: self.properties.clone()
                };
                let mux = Mux::spawn(transport, local_state, local_open, remote_header, self.buffer_size)?;

                // open Connection
                let mut connection = Connection::from(mux);
                connection.mux_mut().control_mut().send(mux::MuxControl::Open).await?;
                Ok(connection)
            },
            // TLS
            "amqps" => {
                todo!()
            },
            _ => return Err(EngineError::Message("Invalid Url Scheme"))
        }
    }

    // pub async fn open_tls<Io>(&self, url: impl TryInto<Url, Error=url::ParseError>) -> Result<Connection<Io>, EngineError> 
    // where 
    //     Io: AsyncRead + AsyncWrite + Unpin,
    // {
    //     todo!()
    // }

    pub async fn pipelined_open(&self, url: impl TryInto<Url>) -> Result<Connection, EngineError> {
        todo!()
    }
}