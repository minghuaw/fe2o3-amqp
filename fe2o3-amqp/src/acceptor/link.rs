//! Link Listener

// /// Listener for incoming link
// #[derive(Debug)]
// pub struct LinkListener {}

use std::marker::PhantomData;

use fe2o3_amqp_types::{
    definitions::{Fields, Role},
    performatives::Attach,
    primitives::{Symbol, ULong},
};
use tracing::instrument;

use crate::{connection::DEFAULT_OUTGOING_BUFFER_SIZE, session::SessionHandle, util::Initialized};

use super::{
    builder::Builder, error::AcceptorAttachError, local_receiver_link::LocalReceiverLinkAcceptor,
    local_sender_link::LocalSenderLinkAcceptor, session::ListenerSessionHandle,
};

/// Listener side link endpoint
#[derive(Debug)]
pub enum LinkEndpoint {
    /// Sender
    Sender(crate::link::Sender),

    /// Receiver
    Receiver(crate::link::Receiver),
}

#[derive(Debug, Clone)]
pub(crate) struct SharedLinkAcceptorFields {
    /// The maximum message size supported by the link endpoint
    pub max_message_size: Option<ULong>,

    /// Link properties
    pub properties: Option<Fields>,

    /// Buffer size for the underlying `mpsc:channel`
    pub buffer_size: usize,

    /// The extension capabilities the sender supports
    pub offered_capabilities: Option<Vec<Symbol>>,

    /// The extension capabilities the sender can use if the receiver supports them
    pub desired_capabilities: Option<Vec<Symbol>>,
}

impl Default for SharedLinkAcceptorFields {
    fn default() -> Self {
        Self {
            buffer_size: DEFAULT_OUTGOING_BUFFER_SIZE,
            max_message_size: None,
            properties: None,
            offered_capabilities: None,
            desired_capabilities: None,
        }
    }
}

/// An acceptor for incoming links
///
/// # Accepts incoming link with default configuration
///
/// ```rust,ignore
/// use crate::acceptor::{ListenerSessionHandle, LinkAcceptor, LinkEndpoint};
///
/// let mut session: ListenerSessionHandle = session_acceptor.accept(&mut connection).await.unwrap();
/// let link_acceptor = LinkAcceptor::new();
/// let link: LinkEndpoint = link_acceptor.accept(&mut session).await.unwrap();
/// ```
///
/// ## Default configuration
///
/// | Field | Default Value |
/// |-------|---------------|
/// |`supported_snd_settle_modes`|[`SupportedSenderSettleModes::All`]|
/// |`fallback_snd_settle_mode`| `None` |
/// |`supported_rcv_settle_modes`|[`SupportedReceiverSettleModes::Both`]|
/// |`fallback_rcv_settle_mode`| `None` |
/// |`initial_delivery_count`| `0` |
/// |`max_message_size`| `None` |
/// |`offered_capabilities`| `None` |
/// |`desired_capabilities`| `None` |
/// |`properties`| `None` |
/// |`buffer_size`| [`u16::MAX`] |
/// |`credit_mode`| [`CreditMode::Auto(DEFAULT_CREDIT)`] |
///
/// # Customize acceptor
///
/// The acceptor can be customized using the builder pattern or by directly
/// modifying the field after the acceptor is built.
///
/// ```rust
/// use crate::acceptor::{LinkAcceptor, SupportedSenderSettleModes};
///
/// let link_acceptor = LinkAcceptor::builder()
///     .supported_sender_settle_modes(SupportedSenderSettleModes::Settled)
///     .build();
/// ```
///
#[derive(Debug, Clone, Default)]
pub struct LinkAcceptor {
    pub(crate) shared: SharedLinkAcceptorFields,
    pub(crate) local_sender_acceptor: LocalSenderLinkAcceptor<Symbol>,
    pub(crate) local_receiver_acceptor: LocalReceiverLinkAcceptor<Symbol>,
}

impl std::fmt::Display for LinkAcceptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("LinkAcceptor"))
    }
}

impl LinkAcceptor {
    /// Creates a default LinkAcceptor
    pub fn new() -> Self {
        Default::default()
    }

    /// Creates a builder for [`LinkAcceptor`]
    pub fn builder() -> Builder<Self, Initialized> {
        Builder::<Self, Initialized>::new()
    }

    /// Convert the acceptor into a link acceptor builder. This allows users to configure
    /// particular field using the builder pattern
    pub fn into_builder(self) -> Builder<Self, Initialized> {
        Builder {
            inner: self,
            marker: PhantomData,
        }
    }

    /// Accept incoming link with an explicit Attach performative
    #[instrument(skip_all)]
    pub async fn accept_incoming_attach<R>(
        &self,
        remote_attach: Attach,
        session: &mut SessionHandle<R>,
    ) -> Result<LinkEndpoint, AcceptorAttachError> {
        // In this case, the sender is considered to hold the authoritative version of the
        // source properties, the receiver is considered to hold the authoritative version of the target properties.
        match remote_attach.role {
            Role::Sender => {
                // Remote is sender -> local is receiver
                self.local_receiver_acceptor
                    .accept_incoming_attach(&self.shared, remote_attach, session)
                    .await
                    .map(|receiver| LinkEndpoint::Receiver(receiver))
                    .map_err(Into::into)
            }
            Role::Receiver => self
                .local_sender_acceptor
                .accept_incoming_attach(&self.shared, remote_attach, session)
                .await
                .map(|sender| LinkEndpoint::Sender(sender))
                .map_err(Into::into),
        }
    }

    /// Accept incoming link by waiting for an incoming Attach performative
    pub async fn accept(
        &self,
        session: &mut ListenerSessionHandle,
    ) -> Result<LinkEndpoint, AcceptorAttachError> {
        let remote_attach = session
            .next_incoming_attach()
            .await
            .ok_or(AcceptorAttachError::IllegalSessionState)?;
        self.accept_incoming_attach(remote_attach, session).await
    }
}
