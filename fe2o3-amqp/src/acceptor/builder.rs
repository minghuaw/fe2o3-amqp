//! Builder for acceptors

use std::marker::PhantomData;

use fe2o3_amqp_types::{
    definitions::{
        Fields, Handle, IetfLanguageTag, Milliseconds, ReceiverSettleMode, SenderSettleMode,
        SequenceNo, TransferNumber, MIN_MAX_FRAME_SIZE,
    },
    messaging::{Source, Target},
    performatives::{ChannelMax, MaxFrameSize, Open},
    primitives::{Array, Symbol, Ulong},
};

use crate::{
    connection::{DEFAULT_CHANNEL_MAX, DEFAULT_MAX_FRAME_SIZE, DEFAULT_OUTGOING_BUFFER_SIZE},
    util::{Initialized, Uninitialized},
};

use super::{
    link::LinkAcceptor, local_receiver_link::LocalReceiverLinkAcceptor,
    local_sender_link::LocalSenderLinkAcceptor, session::SessionAcceptor, ConnectionAcceptor,
    SaslAcceptor, SupportedReceiverSettleModes, SupportedSenderSettleModes,
};

cfg_transaction! {
    use fe2o3_amqp_types::transaction::TxnCapability;
    
    use crate::transaction::coordinator::ControlLinkAcceptor;
}

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

impl Default for Builder<ConnectionAcceptor<(), ()>, Uninitialized> {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder<ConnectionAcceptor<(), ()>, Uninitialized> {
    /// Creates a new Builder for `ConnectionAccptor`
    pub fn new() -> Self {
        let local_open = Open {
            container_id: String::with_capacity(0), // This is going to be changed
            hostname: None,
            max_frame_size: MaxFrameSize(DEFAULT_MAX_FRAME_SIZE),
            channel_max: ChannelMax(DEFAULT_CHANNEL_MAX),
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
        mut self,
        id: impl Into<String>,
    ) -> Builder<ConnectionAcceptor<Tls, Sasl>, Initialized> {
        self.inner.local_open.container_id = id.into();

        Builder {
            inner: self.inner,
            marker: PhantomData,
        }
    }

    /// The name of the target host
    pub fn hostname(mut self, hostname: impl Into<Option<String>>) -> Self {
        self.inner.local_open.hostname = hostname.into();
        self
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
    /// number of sessions that can be simultaenously active on the connection
    pub fn session_max(mut self, session_max: impl Into<ChannelMax>) -> Self {
        let mut channel_max = session_max.into();
        channel_max.0 = channel_max.0.saturating_sub(1);
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
            Some(locales) => locales.0.push(locale.into()),
            None => self.inner.local_open.outgoing_locales = Some(vec![locale.into()].into()),
        }
        self
    }

    /// Set the locales available for outgoing text
    pub fn set_outgoing_locales(mut self, locales: impl Into<Array<IetfLanguageTag>>) -> Self {
        self.inner.local_open.outgoing_locales = Some(locales.into());
        self
    }

    /// Add one desired locales for incoming text in decreasing level of preference
    pub fn add_incoming_locales(mut self, locale: impl Into<IetfLanguageTag>) -> Self {
        match &mut self.inner.local_open.incoming_locales {
            Some(locales) => locales.0.push(locale.into()),
            None => self.inner.local_open.incoming_locales = Some(vec![locale.into()].into()),
        }
        self
    }

    /// Set the desired locales for incoming text in decreasing level of preference
    pub fn set_incoming_locales(mut self, locales: impl Into<Array<IetfLanguageTag>>) -> Self {
        self.inner.local_open.incoming_locales = Some(locales.into());
        self
    }

    /// Add one extension capabilities the sender supports
    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.local_open.offered_capabilities {
            Some(capabilities) => capabilities.0.push(capability.into()),
            None => {
                self.inner.local_open.offered_capabilities = Some(vec![capability.into()].into())
            }
        }
        self
    }

    /// Set the extension capabilities the sender supports
    pub fn set_offered_capabilities(mut self, capabilities: impl Into<Array<Symbol>>) -> Self {
        self.inner.local_open.offered_capabilities = Some(capabilities.into());
        self
    }

    /// Add one extension capabilities the sender can use if the receiver supports them
    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.local_open.desired_capabilities {
            Some(capabilities) => capabilities.0.push(capability.into()),
            None => {
                self.inner.local_open.desired_capabilities = Some(vec![capability.into()].into())
            }
        }
        self
    }

    /// Set the extension capabilities the sender can use if the receiver supports them
    pub fn set_desired_capabilities(mut self, capabilities: impl Into<Array<Symbol>>) -> Self {
        self.inner.local_open.desired_capabilities = Some(capabilities.into());
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
    pub fn sasl_acceptor<S>(self, sasl_acceptor: S) -> Builder<ConnectionAcceptor<Tls, S>, M>
    where
        S: SaslAcceptor,
    {
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

// =============================================================================
// SessionAcceptor builder
// =============================================================================

impl Default for Builder<SessionAcceptor, Initialized> {
    fn default() -> Self {
        Self::new()
    }
}

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
    pub fn outgoing_window(mut self, value: TransferNumber) -> Self {
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

    cfg_transaction! {
        /// Enable handling remotely initiated control link and transaction by setting the
        /// `control_link_acceptor` field
        pub fn control_link_acceptor(
            mut self,
            control_link_acceptor: impl Into<Option<ControlLinkAcceptor>>,
        ) -> Self {
            self.inner.0.control_link_acceptor = control_link_acceptor.into();
            self
        }
    }
}

// =============================================================================
// LinkAcceptor builder
// =============================================================================

impl Default
    for Builder<
        LinkAcceptor<fn(Source) -> Option<Source>, fn(Target) -> Option<Target>>,
        Initialized,
    >
{
    fn default() -> Self {
        Self::new()
    }
}

impl
    Builder<LinkAcceptor<fn(Source) -> Option<Source>, fn(Target) -> Option<Target>>, Initialized>
{
    /// Creates a new builder for [`LinkAcceptor`]
    pub fn new() -> Self {
        let inner = LinkAcceptor {
            ..Default::default()
        };

        Self {
            inner,
            marker: PhantomData,
        }
    }
}

impl<FS, FT> Builder<LinkAcceptor<FS, FT>, Initialized>
where
    FS: Fn(Source) -> Option<Source>,
    FT: Fn(Target) -> Option<Target>,
{
    /// Settlement policy for the sender
    pub fn supported_sender_settle_modes(mut self, modes: SupportedSenderSettleModes) -> Self {
        self.inner.shared.supported_snd_settle_modes = modes;
        self
    }

    /// The sender settle mode to fallback to when the mode desired
    /// by the remote peer is not supported
    pub fn fallback_sender_settle_mode(mut self, mode: SenderSettleMode) -> Self {
        self.inner.shared.fallback_snd_settle_mode = mode;
        self
    }

    /// The settlement policy of the receiver
    pub fn supported_receiver_settle_modes(mut self, modes: SupportedReceiverSettleModes) -> Self {
        self.inner.shared.supported_rcv_settle_modes = modes;
        self
    }

    /// The receiver settle mode to fallback to when the mode desired
    /// by the remote peer is not supported
    pub fn fallback_receiver_settle_mode(mut self, mode: ReceiverSettleMode) -> Self {
        self.inner.shared.fallback_rcv_settle_mode = mode;
        self
    }

    /// This MUST NOT be null if role is sender,
    /// and it is ignored if the role is receiver.
    /// See subsection 2.6.7.
    pub fn initial_delivery_count(mut self, count: SequenceNo) -> Self {
        self.inner.local_sender_acceptor.initial_delivery_count = count;
        self
    }

    /// The maximum message size supported by the link endpoint
    pub fn max_message_size(mut self, max_size: impl Into<Ulong>) -> Self {
        self.inner.shared.max_message_size = Some(max_size.into());
        self
    }

    /// Add one extension capability the sender supports
    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.shared.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.inner.shared.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender supports
    pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.inner.shared.offered_capabilities = Some(capabilities);
        self
    }

    /// Add one extension capability the sender can use if the receiver supports
    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.inner.shared.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.inner.shared.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender can use if the receiver supports them
    pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.inner.shared.desired_capabilities = Some(capabilities);
        self
    }

    /// Link properties
    pub fn properties(mut self, properties: Fields) -> Self {
        self.inner.shared.properties = Some(properties);
        self
    }

    /// Set the target capabilities field
    pub fn target_capabilities(
        mut self,
        target_capabilities: impl Into<Option<Vec<Symbol>>>,
    ) -> Self {
        self.inner.local_receiver_acceptor.target_capabilities = target_capabilities.into();
        self
    }

    /// Set the source capabilities field
    pub fn source_capabilities(
        mut self,
        source_capabilities: impl Into<Option<Vec<Symbol>>>,
    ) -> Self {
        self.inner.local_sender_acceptor.source_capabilities = source_capabilities.into();
        self
    }

    /// Set whether the link should verify the `source` field of incoming Attach frames
    pub fn verify_incoming_source(mut self, verify: bool) -> Self {
        self.inner.local_receiver_acceptor.verify_incoming_source = verify;
        self.inner.local_sender_acceptor.verify_incoming_source = verify;
        self
    }

    /// Set whether the link should verify the `target` field of incoming Attach frames
    pub fn verify_incoming_target(mut self, verify: bool) -> Self {
        self.inner.local_sender_acceptor.verify_incoming_target = verify;
        self.inner.local_receiver_acceptor.verify_incoming_target = verify;
        self
    }

    /// Sets how to handle dynamic target
    ///
    /// If a valid target is created, a `Some(target)` should be returned. If dynamic
    /// node creation is not supported, then a `None` should be returned.
    ///
    /// The default handler simply rejects the request by returning a `None`
    pub fn on_dynamic_target<F>(self, op: F) -> Builder<LinkAcceptor<FS, F>, Initialized>
    where
        F: Fn(Target) -> Option<Target>,
    {
        let local_receiver_acceptor = LocalReceiverLinkAcceptor {
            credit_mode: self.inner.local_receiver_acceptor.credit_mode,
            target_capabilities: self.inner.local_receiver_acceptor.target_capabilities,
            auto_accept: self.inner.local_receiver_acceptor.auto_accept,
            on_dynamic_target: op,
            target_marker: PhantomData,
            verify_incoming_source: self.inner.local_receiver_acceptor.verify_incoming_source,
            verify_incoming_target: self.inner.local_receiver_acceptor.verify_incoming_target,
        };
        let inner = LinkAcceptor {
            shared: self.inner.shared,
            local_sender_acceptor: self.inner.local_sender_acceptor,
            local_receiver_acceptor,
        };

        Builder {
            inner,
            marker: PhantomData,
        }
    }

    /// Sets how to handle dynamic source
    ///
    /// If a valid source is created, a `Some(source)` should be returned. If dynamic
    /// node creation is not supported, then a `None` should be returned.
    ///
    /// The default handler simply rejects the request by returning a `None`
    pub fn on_dynamic_source<F>(self, op: F) -> Builder<LinkAcceptor<F, FT>, Initialized>
    where
        F: Fn(Source) -> Option<Source>,
    {
        let local_sender_acceptor = LocalSenderLinkAcceptor {
            initial_delivery_count: self.inner.local_sender_acceptor.initial_delivery_count,
            source_capabilities: self.inner.local_sender_acceptor.source_capabilities,
            on_dynamic_source: op,
            verify_incoming_source: self.inner.local_sender_acceptor.verify_incoming_source,
            verify_incoming_target: self.inner.local_sender_acceptor.verify_incoming_target,
        };
        let inner = LinkAcceptor {
            shared: self.inner.shared,
            local_sender_acceptor,
            local_receiver_acceptor: self.inner.local_receiver_acceptor,
        };

        Builder {
            inner,
            marker: PhantomData,
        }
    }
}

// =============================================================================
// ControlLinkAcceptor
// =============================================================================

cfg_transaction! {
    impl Builder<ControlLinkAcceptor, Initialized> {
        /// Creates a builder for `ControlLinkAcceptor`
        pub fn new() -> Self {
            let shared = Default::default();
            let inner = Default::default();
            let inner = ControlLinkAcceptor { shared, inner };
    
            Self {
                inner,
                marker: PhantomData,
            }
        }
    
        /// Settlement policy for the sender
        pub fn supported_sender_settle_modes(mut self, modes: SupportedSenderSettleModes) -> Self {
            self.inner.shared.supported_snd_settle_modes = modes;
            self
        }
    
        /// The sender settle mode to fallback to when the mode desired
        /// by the remote peer is not supported
        pub fn fallback_sender_settle_mode(mut self, mode: SenderSettleMode) -> Self {
            self.inner.shared.fallback_snd_settle_mode = mode;
            self
        }
    
        /// The settlement policy of the receiver
        pub fn supported_receiver_settle_modes(mut self, modes: SupportedReceiverSettleModes) -> Self {
            self.inner.shared.supported_rcv_settle_modes = modes;
            self
        }
    
        /// The receiver settle mode to fallback to when the mode desired
        /// by the remote peer is not supported
        pub fn fallback_receiver_settle_mode(mut self, mode: ReceiverSettleMode) -> Self {
            self.inner.shared.fallback_rcv_settle_mode = mode;
            self
        }
    
        /// The maximum message size supported by the link endpoint
        pub fn max_message_size(mut self, max_size: impl Into<Ulong>) -> Self {
            self.inner.shared.max_message_size = Some(max_size.into());
            self
        }
    
        /// Add one extension capability the sender supports
        pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
            match &mut self.inner.shared.offered_capabilities {
                Some(capabilities) => capabilities.push(capability.into()),
                None => self.inner.shared.offered_capabilities = Some(vec![capability.into()]),
            }
            self
        }
    
        /// Set the extension capabilities the sender supports
        pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
            self.inner.shared.offered_capabilities = Some(capabilities);
            self
        }
    
        /// Add one extension capability the sender can use if the receiver supports
        pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
            match &mut self.inner.shared.desired_capabilities {
                Some(capabilities) => capabilities.push(capability.into()),
                None => self.inner.shared.desired_capabilities = Some(vec![capability.into()]),
            }
            self
        }
    
        /// Set the extension capabilities the sender can use if the receiver supports them
        pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
            self.inner.shared.desired_capabilities = Some(capabilities);
            self
        }
    
        /// Link properties
        pub fn properties(mut self, properties: Fields) -> Self {
            self.inner.shared.properties = Some(properties);
            self
        }
    
        /// Set the target capabilities field
        pub fn target_capabilities(
            mut self,
            target_capabilities: impl Into<Option<Vec<TxnCapability>>>,
        ) -> Self {
            self.inner.inner.target_capabilities = target_capabilities.into();
            self
        }
    }
}
