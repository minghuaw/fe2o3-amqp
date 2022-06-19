//! Link Listener

// /// Listener for incoming link
// #[derive(Debug)]
// pub struct LinkListener {}

use std::marker::PhantomData;

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{self, AmqpError, Fields, Role},
    messaging::{DeliveryState, TargetArchetype},
    performatives::Attach,
    primitives::{Symbol, ULong},
};
use tokio::sync::mpsc;

use crate::{
    connection::DEFAULT_OUTGOING_BUFFER_SIZE,
    control::SessionControl,
    endpoint,
    link::{
        role, state::LinkFlowState, target_archetype::VerifyTargetArchetype, AttachError, Link,
        LinkFrame,
    },
    session::SessionHandle,
    util::Initialized,
};

use super::{
    builder::Builder, local_receiver_link::LocalReceiverLinkAcceptor,
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
    pub async fn accept_incoming_attach<R>(
        &self,
        remote_attach: Attach,
        session: &mut SessionHandle<R>,
    ) -> Result<LinkEndpoint, AttachError> {
        // In this case, the sender is considered to hold the authoritative version of the
        // source properties, the receiver is considered to hold the authoritative version of the target properties.
        let result = match remote_attach.role {
            Role::Sender => {
                // Remote is sender -> local is receiver
                self.local_receiver_acceptor
                    .accept_incoming_attach(&self.shared, remote_attach, session)
                    .await
                    .map(|receiver| LinkEndpoint::Receiver(receiver))
            }
            Role::Receiver => self
                .local_sender_acceptor
                .accept_incoming_attach(&self.shared, remote_attach, session)
                .await
                .map(|sender| LinkEndpoint::Sender(sender)),
        };

        match result {
            Ok(link) => Ok(link),
            Err((error, remote_attach)) => {
                Err(
                    handle_attach_error(error, remote_attach, &session.outgoing, &session.control)
                        .await,
                )
            }
        }
    }

    /// Accept incoming link by waiting for an incoming Attach performative
    pub async fn accept(
        &self,
        session: &mut ListenerSessionHandle,
    ) -> Result<LinkEndpoint, AttachError> {
        let remote_attach = session
            .next_incoming_attach()
            .await
            .ok_or_else(|| AttachError::IllegalSessionState)?;
        self.accept_incoming_attach(remote_attach, session).await
    }
}

/// Reject an incoming attach with an attach that has either target
/// or source field left empty (None or Null)
pub(crate) async fn reject_incoming_attach(
    mut remote_attach: Attach,
    outgoing: &mpsc::Sender<LinkFrame>,
) -> Result<(), AttachError> {
    let local_attach = match remote_attach.role {
        Role::Sender => {
            remote_attach.target = None;
            remote_attach
        }
        Role::Receiver => {
            remote_attach.source = None;
            remote_attach
        }
    };
    let frame = LinkFrame::Attach(local_attach);
    outgoing
        .send(frame)
        .await
        .map_err(|_| AttachError::IllegalSessionState)?; // Session must have been dropped
    Ok(())
}

/// If remote_attach is some, then the link should echo an attach with emtpy source or target
pub(crate) async fn handle_attach_error(
    error: AttachError,
    remote_attach: Option<Attach>,
    outgoing: &mpsc::Sender<LinkFrame>,
    session_control: &mpsc::Sender<SessionControl>,
) -> AttachError {
    // If a response of an empty attach is needed
    if let Some(remote_attach) = remote_attach {
        if let Err(err) = reject_incoming_attach(remote_attach, outgoing).await {
            return err
        }
    }

    // Additional handling
    match error {
        AttachError::IllegalSessionState => {
            let err = definitions::Error::new(
                AmqpError::IllegalState,
                "Illegal session state".to_string(),
                None,
            );
            if let Err(_) = session_control.send(SessionControl::End(Some(err))).await {
                return AttachError::IllegalSessionState
            }
            error
        }
        AttachError::HandleMaxReached // TODO: any additional steps needed?
        | AttachError::DuplicatedLinkName
        | AttachError::SourceIsNone
        | AttachError::TargetIsNone
        | AttachError::Local(_) => error,
    }
}

#[async_trait]
impl<R, T, F, M> endpoint::LinkAttachAcceptorExt for Link<R, T, F, M>
where
    R: role::IntoRole + Send + Sync,
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
    F: AsRef<LinkFlowState<R>> + Send + Sync,
    M: AsRef<DeliveryState> + AsMut<DeliveryState> + Send + Sync,
{
    async fn on_incoming_attach_as_acceptor(
        &mut self,
        mut remote_attach: Attach,
    ) -> Result<(), (Self::AttachError, Option<Attach>)> {
        self.on_incoming_attach_inner(
            // Cloning is relatively cheap except for on target and source
            remote_attach.handle.clone(),
            remote_attach.target.take(),
            remote_attach.role.clone(),
            remote_attach.source.take(),
            remote_attach.snd_settle_mode.clone(),
            remote_attach.initial_delivery_count.clone(),
            remote_attach.rcv_settle_mode.clone(),
            remote_attach.max_message_size.clone(),
        )
        .await
        .map_err(|err| (err, Some(remote_attach)))
    }
}
