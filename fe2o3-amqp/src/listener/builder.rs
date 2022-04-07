//! Builder for acceptors

use std::marker::PhantomData;

use fe2o3_amqp_types::{performatives::{Open, MaxFrameSize, ChannelMax}, definitions::{MIN_MAX_FRAME_SIZE, Milliseconds, IetfLanguageTag, Fields}, primitives::Symbol};

use crate::{util::{Uninitialized, Initialized}, connection::DEFAULT_OUTGOING_BUFFER_SIZE};

use super::ConnectionAcceptor;

/// A generic builder for listener connection, session and link acceptors
#[derive(Debug)]
pub struct Builder<T, M> {
    inner: T,
    marker: PhantomData<M>,
}

impl Builder<ConnectionAcceptor<()>, Uninitialized> {
    /// Creates a new Builder for [`ConnectionAccptor`]
    pub fn new() -> Self {
        let local_open = Open {
            container_id: String::with_capacity(0), // This is going to be changed
            hostname: None,
            max_frame_size: Default::default(),
            channel_max: Default::default(),
            idle_time_out: None,
            outgoing_locales: None,
            incoming_locales: None,
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,
        };

        let inner = ConnectionAcceptor {
            local_open,
            tls_acceptor: (),
            sasl_mechanisms: Vec::new(),
            buffer_size: DEFAULT_OUTGOING_BUFFER_SIZE,
        };

        Self {
            inner,
            marker: PhantomData,
        }
    }
}

impl<M, Tls> Builder<ConnectionAcceptor<Tls>, M> {
    /// The id of the source container
    pub fn container_id(self, id: impl Into<String>) -> Builder<ConnectionAcceptor<Tls>, Initialized> {
        // In Rust, it’s more common to pass slices as arguments
        // rather than vectors when you just want to provide read access.
        // The same goes for String and &str.
        let local_open = Open {
            container_id: id.into(),
            hostname: self.inner.local_open.hostname,
            max_frame_size: self.inner.local_open.max_frame_size,
            channel_max: self.inner.local_open.channel_max,
            idle_time_out: self.inner.local_open.idle_time_out,
            outgoing_locales: self.inner.local_open.outgoing_locales,
            incoming_locales: self.inner.local_open.incoming_locales,
            offered_capabilities: self.inner.local_open.offered_capabilities,
            desired_capabilities: self.inner.local_open.desired_capabilities,
            properties: self.inner.local_open.properties,
        };

        let inner = ConnectionAcceptor {
            local_open,
            tls_acceptor: self.inner.tls_acceptor,
            sasl_mechanisms: self.inner.sasl_mechanisms,
            buffer_size: self.inner.buffer_size
        };

        Builder { inner, marker: PhantomData }
    }

    /// Proposed maximum frame size
    pub fn max_frame_size(mut self, max_frame_size: impl Into<MaxFrameSize>) -> Self {
        let max_frame_size = max_frame_size.into();
        let max_frame_size = std::cmp::max(MIN_MAX_FRAME_SIZE as u32, max_frame_size.0);
        self.inner.local_open
            .max_frame_size = MaxFrameSize::from(max_frame_size);
        self
    }

    /// The maximum channel number that can be used on the connection
    ///
    /// The channel-max value is the highest channel number that can be used on the connection. This
    /// value plus one is the maximum number of sessions that can be simultaneously active on the
    /// connection
    pub fn channel_max(mut self, channel_max: impl Into<ChannelMax>) -> Self {
        self.inner.local_open
            .channel_max = channel_max.into();
        self
    }

    /// The maximum number of session that can be established on this connection.
    ///
    /// This will modify the `channel-max` field. The `channel-max` plus one is the maximum
    /// number of sessions taht can be simultaenously active on the connection
    pub fn session_max(mut self, session_max: impl Into<ChannelMax>) -> Self {
        let mut channel_max = session_max.into();
        channel_max.0 -= 1;
        self.inner.local_open
            .channel_max = channel_max;
        self
    }

    /// Idle time-out
    pub fn idle_time_out(mut self, idle_time_out: impl Into<Milliseconds>) -> Self {
        self.inner.local_open
            .idle_time_out = Some(idle_time_out.into());
        self
    }

    /// Add one locales available for outgoing text
    pub fn add_outgoing_locales(mut self, locale: impl Into<IetfLanguageTag>) -> Self {
        match &mut self.inner.local_open
            .outgoing_locales {
            Some(locales) => locales.push(locale.into()),
            None => self.inner.local_open
                .outgoing_locales = Some(vec![locale.into()]),
        }
        self
    }

    /// Set the locales available for outgoing text
    pub fn set_outgoing_locales(mut self, locales: Vec<IetfLanguageTag>) -> Self {
        self.inner.local_open
            .outgoing_locales = Some(locales);
        self
    }

    /// Add one desired locales for incoming text in decreasing level of preference
    pub fn add_incoming_locales(mut self, locale: impl Into<IetfLanguageTag>) -> Self {
        match &mut self.inner.local_open
            .incoming_locales {
            Some(locales) => locales.push(locale.into()),
            None => self.inner.local_open
                .incoming_locales = Some(vec![locale.into()]),
        }
        self
    }

    /// Set the desired locales for incoming text in decreasing level of preference
    pub fn set_incoming_locales(mut self, locales: Vec<IetfLanguageTag>) -> Self {
        self.inner.local_open
            .incoming_locales = Some(locales);
        self
    }

    /// Add one extension capabilities the sender supports
    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.local_open
            .offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.inner.local_open
                .offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender supports
    pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.inner.local_open
            .offered_capabilities = Some(capabilities);
        self
    }

    /// Add one extension capabilities the sender can use if the receiver supports them
    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.local_open
            .desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.inner.local_open.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender can use if the receiver supports them
    pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.inner.local_open
            .desired_capabilities = Some(capabilities);
        self
    }

    /// Connection properties
    pub fn properties(mut self, properties: Fields) -> Self {
        self.inner.local_open
            .properties = Some(properties);
        self
    }

    /// Buffer size of the underlying [`tokio::sync::mpsc::channel`] that are used by the sessions
    pub fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.inner.buffer_size = buffer_size;
        self
    }
}

impl<Tls> Builder<ConnectionAcceptor<Tls>, Initialized> {
    /// Build [`ConnectionAcceptor`]
    pub fn build(self) -> ConnectionAcceptor<Tls> {
        self.inner
    }
}