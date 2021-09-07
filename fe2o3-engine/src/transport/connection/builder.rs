use std::convert::TryInto;

use fe2o3_amqp::primitives::{Symbol};
use fe2o3_types::{definitions::{Fields, IetfLanguageTag, Milliseconds}, performatives::{ChannelMax, MaxFrameSize}};
use tokio::net::TcpStream;
use url::Url;

use crate::error::EngineError;

use super::Connection;


/// Connection builder
pub struct Builder {
    pub container_id: String,
    pub hostname: Option<String>,
    pub max_frame_size: MaxFrameSize,
    pub channel_max: ChannelMax,
    pub idle_time_out: Option<Milliseconds>,
    pub outgoing_locales: Option<Vec<IetfLanguageTag>>,
    pub incoming_locales: Option<Vec<IetfLanguageTag>>,
    pub offered_capabilities: Option<Vec<Symbol>>,
    pub desired_capabilities: Option<Vec<Symbol>>,
    pub properties: Option<Fields>
}

impl Builder {
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
            properties: None
        }
    }

    // In Rust, it’s more common to pass slices as arguments 
    // rather than vectors when you just want to provide read access. 
    // The same goes for String and &str.
    pub fn container_id(&mut self, id: String) -> &mut Self {
        self.container_id = id;
        self
    }

    pub fn hostname(&mut self, hostname: String) -> &mut Self {
        self.hostname = Some(hostname);
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

    pub fn idle_time_out(&mut self, idle_time_out: Milliseconds) -> &mut Self {
        self.idle_time_out = Some(idle_time_out);
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

    pub fn build_connection(address: impl TryInto<Url>) -> Result<Connection<TcpStream>, EngineError> {
        todo!()
    }
}