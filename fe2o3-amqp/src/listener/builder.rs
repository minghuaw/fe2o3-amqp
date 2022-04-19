//! Builder for acceptors

use std::marker::PhantomData;

use fe2o3_amqp_types::{
    definitions::{
        Fields, Handle, IetfLanguageTag, Milliseconds, TransferNumber, MIN_MAX_FRAME_SIZE, SenderSettleMode, ReceiverSettleMode, SequenceNo,
    },
    performatives::{ChannelMax, MaxFrameSize, Open},
    primitives::{Symbol, ULong},
};

use crate::{
    connection::DEFAULT_OUTGOING_BUFFER_SIZE,
    util::{Initialized, Uninitialized},
};

use super::{session::SessionAcceptor, ConnectionAcceptor, link::LinkAcceptor, SupportedSenderSettleModes, SupportedReceiverSettleModes};

/// A generic builder for listener connection, session and link acceptors
#[derive(Debug)]
pub struct Builder<T, M> {
    pub(crate) inner: T,
    pub(crate) marker: PhantomData<M>,
}

impl<T> Builder<T, Initialized> {
    /// Build the instance
    pub fn build(self) -> T {
        self.inner
    }
}

// =============================================================================
// ConnectionAccpetor builder
// =============================================================================

impl Builder<ConnectionAcceptor<(), ()>, Uninitialized> {
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
            sasl_acceptor: (),
            buffer_size: DEFAULT_OUTGOING_BUFFER_SIZE,
        };

        Self {
            inner,
            marker: PhantomData,
        }
    }
}

impl<M, Tls, Sasl> Builder<ConnectionAcceptor<Tls, Sasl>, M> {
    /// The id of the source container
    pub fn container_id(
        self,
        id: impl Into<String>,
    ) -> Builder<ConnectionAcceptor<Tls, Sasl>, Initialized> {
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
            sasl_acceptor: self.inner.sasl_acceptor,
            buffer_size: self.inner.buffer_size,
        };

        Builder {
            inner,
            marker: PhantomData,
        }
    }

    /// Proposed maximum frame size
    pub fn max_frame_size(mut self, max_frame_size: impl Into<MaxFrameSize>) -> Self {
        let max_frame_size = max_frame_size.into();
        let max_frame_size = std::cmp::max(MIN_MAX_FRAME_SIZE as u32, max_frame_size.0);
        self.inner.local_open.max_frame_size = MaxFrameSize::from(max_frame_size);
        self
    }

    /// The maximum channel number that can be used on the connection
    ///
    /// The channel-max value is the highest channel number that can be used on the connection. This
    /// value plus one is the maximum number of sessions that can be simultaneously active on the
    /// connection
    pub fn channel_max(mut self, channel_max: impl Into<ChannelMax>) -> Self {
        self.inner.local_open.channel_max = channel_max.into();
        self
    }

    /// The maximum number of session that can be established on this connection.
    ///
    /// This will modify the `channel-max` field. The `channel-max` plus one is the maximum
    /// number of sessions taht can be simultaenously active on the connection
    pub fn session_max(mut self, session_max: impl Into<ChannelMax>) -> Self {
        let mut channel_max = session_max.into();
        channel_max.0 -= 1;
        self.inner.local_open.channel_max = channel_max;
        self
    }

    /// Idle time-out
    pub fn idle_time_out(mut self, idle_time_out: impl Into<Milliseconds>) -> Self {
        self.inner.local_open.idle_time_out = Some(idle_time_out.into());
        self
    }

    /// Add one locales available for outgoing text
    pub fn add_outgoing_locales(mut self, locale: impl Into<IetfLanguageTag>) -> Self {
        match &mut self.inner.local_open.outgoing_locales {
            Some(locales) => locales.push(locale.into()),
            None => self.inner.local_open.outgoing_locales = Some(vec![locale.into()]),
        }
        self
    }

    /// Set the locales available for outgoing text
    pub fn set_outgoing_locales(mut self, locales: Vec<IetfLanguageTag>) -> Self {
        self.inner.local_open.outgoing_locales = Some(locales);
        self
    }

    /// Add one desired locales for incoming text in decreasing level of preference
    pub fn add_incoming_locales(mut self, locale: impl Into<IetfLanguageTag>) -> Self {
        match &mut self.inner.local_open.incoming_locales {
            Some(locales) => locales.push(locale.into()),
            None => self.inner.local_open.incoming_locales = Some(vec![locale.into()]),
        }
        self
    }

    /// Set the desired locales for incoming text in decreasing level of preference
    pub fn set_incoming_locales(mut self, locales: Vec<IetfLanguageTag>) -> Self {
        self.inner.local_open.incoming_locales = Some(locales);
        self
    }

    /// Add one extension capabilities the sender supports
    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.local_open.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.inner.local_open.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender supports
    pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.inner.local_open.offered_capabilities = Some(capabilities);
        self
    }

    /// Add one extension capabilities the sender can use if the receiver supports them
    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.local_open.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.inner.local_open.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender can use if the receiver supports them
    pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.inner.local_open.desired_capabilities = Some(capabilities);
        self
    }

    /// Connection properties
    pub fn properties(mut self, properties: Fields) -> Self {
        self.inner.local_open.properties = Some(properties);
        self
    }

    /// Sets the TLS Acceptor
    pub fn tls_acceptor<T>(self, tls_acceptor: T) -> Builder<ConnectionAcceptor<T, Sasl>, M> {
        let inner = ConnectionAcceptor {
            local_open: self.inner.local_open,
            tls_acceptor,
            sasl_acceptor: self.inner.sasl_acceptor,
            buffer_size: self.inner.buffer_size,
        };
        Builder {
            inner,
            marker: PhantomData,
        }
    }

    /// Sets the SASL acceptor
    pub fn sasl_acceptor<S>(self, sasl_acceptor: S) -> Builder<ConnectionAcceptor<Tls, S>, M> {
        let inner = ConnectionAcceptor {
            local_open: self.inner.local_open,
            tls_acceptor: self.inner.tls_acceptor,
            sasl_acceptor,
            buffer_size: self.inner.buffer_size,
        };
        Builder {
            inner,
            marker: PhantomData,
        }
    }

    /// Buffer size of the underlying [`tokio::sync::mpsc::channel`] that are used by the sessions
    pub fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.inner.buffer_size = buffer_size;
        self
    }
}

// impl<T, S> Builder<ConnectionAcceptor<T, S>, Initialized> {
//     /// Build [`ConnectionAcceptor`]
//     pub fn build(self) -> ConnectionAcceptor<T, S> {
//         self.inner
//     }
// }

// =============================================================================
// SessionAcceptor builder
// =============================================================================

impl Builder<SessionAcceptor, Initialized> {
    /// Creates a builder for [`SessionAcceptor`]
    pub fn new() -> Self {
        let session_builder = crate::session::Builder::new();
        let inner = SessionAcceptor(session_builder);
        Self {
            inner,
            marker: PhantomData,
        }
    }

    /// The transfer-id of the first transfer id the sender will send
    pub fn next_outgoing_id(mut self, value: TransferNumber) -> Self {
        self.inner.0.next_outgoing_id = value;
        self
    }

    /// The initial incoming-window of the sender
    pub fn incoming_window(mut self, value: TransferNumber) -> Self {
        self.inner.0.incoming_window = value;
        self
    }

    /// The initial outgoing-window of the sender
    pub fn outgoing_widnow(mut self, value: TransferNumber) -> Self {
        self.inner.0.outgoing_window = value;
        self
    }

    /// The maximum handle value that can be used on the session
    pub fn handle_max(mut self, value: impl Into<Handle>) -> Self {
        self.inner.0.handle_max = value.into();
        self
    }

    /// Add one extension capabilities the sender supports
    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.0.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.inner.0.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender supports
    pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.inner.0.offered_capabilities = Some(capabilities);
        self
    }

    /// Add one extension capabilities the sender can use if the receiver supports them
    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.0.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.inner.0.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender can use if the receiver supports them
    pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.inner.0.desired_capabilities = Some(capabilities);
        self
    }

    /// Session properties
    pub fn properties(mut self, properties: Fields) -> Self {
        self.inner.0.properties = Some(properties);
        self
    }

    /// Buffer size of the underlying [`tokio::sync::mpsc::channel`]
    /// that are used by links attached to the session
    pub fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.inner.0.buffer_size = buffer_size;
        self
    }
}

// =============================================================================
// LinkAcceptor builder
// =============================================================================

impl Builder<LinkAcceptor, Initialized> {
    /// Creates a new builder for [`LinkAcceptor`]
    pub fn new() -> Self {
        let inner = LinkAcceptor {
            supported_snd_settle_modes: Default::default(),
            fallback_snd_settle_mode: Default::default(),
            supported_rcv_settle_modes: Default::default(),
            fallback_rcv_settle_mode: Default::default(),
            initial_delivery_count: Default::default(),
            max_message_size: Default::default(),
            offered_capabilities: Default::default(),
            desired_capabilities: Default::default(),
            properties: Default::default(),
            buffer_size: DEFAULT_OUTGOING_BUFFER_SIZE,
            credit_mode: Default::default(),
        };

        Self {
            inner, 
            marker: PhantomData,
        }
    }

    /// Settlement policy for the sender
    pub fn supported_snd_settle_modes(mut self, modes: SupportedSenderSettleModes) -> Self {
        self.inner.supported_snd_settle_modes = modes;
        self
    }

    /// The sender settle mode to fallback to when the mode desired 
    /// by the remote peer is not supported
    pub fn fallback_snd_settle_mode(mut self, mode: SenderSettleMode) -> Self {
        self.inner.fallback_snd_settle_mode = mode;
        self
    }

    /// The settlement policy of the receiver
    pub fn supported_rcv_settle_modes(mut self, modes: SupportedReceiverSettleModes) -> Self {
        self.inner.supported_rcv_settle_modes = modes;
        self
    }

    /// The receiver settle mode to fallback to when the mode desired 
    /// by the remote peer is not supported
    pub fn fallback_rcv_settle_mode(mut self, mode: ReceiverSettleMode) -> Self {
        self.inner.fallback_rcv_settle_mode = mode;
        self
    }

    /// This MUST NOT be null if role is sender,
    /// and it is ignored if the role is receiver.
    /// See subsection 2.6.7.
    pub fn initial_delivery_count(mut self, count: SequenceNo) -> Self {
        self.inner.initial_delivery_count = count;
        self
    }

    /// The maximum message size supported by the link endpoint
    pub fn max_message_size(mut self, max_size: impl Into<ULong>) -> Self {
        self.inner.max_message_size = Some(max_size.into());
        self
    }

    
    /// Add one extension capability the sender supports
    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.inner.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender supports
    pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.inner.offered_capabilities = Some(capabilities);
        self
    }

    /// Add one extension capability the sender can use if the receiver supports
    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.inner.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender can use if the receiver supports them
    pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.inner.desired_capabilities = Some(capabilities);
        self
    }

    /// Link properties
    pub fn properties(mut self, properties: Fields) -> Self {
        self.inner.properties = Some(properties);
        self
    }
}