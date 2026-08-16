//! Link Listener

// /// Listener for incoming link
// #[derive(Debug)]
// pub struct LinkListener {}

use std::marker::PhantomData;

use fe2o3_amqp_types::{
    definitions::{Fields, ReceiverSettleMode, Role, SenderSettleMode},
    messaging::{Source, Target},
    performatives::Attach,
    primitives::{Symbol, Ulong},
};

use crate::{
    connection::DEFAULT_OUTGOING_BUFFER_SIZE,
    link::SessionStopReason,
    session::SessionHandle,
    util::Initialized,
};

use super::{
    builder::Builder, error::AcceptorAttachError, local_receiver_link::LocalReceiverLinkAcceptor,
    local_sender_link::LocalSenderLinkAcceptor, session::ListenerSessionHandle,
    SupportedReceiverSettleModes, SupportedSenderSettleModes,
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
    pub max_message_size: Option<Ulong>,

    /// Link properties
    pub properties: Option<Fields>,

    /// Buffer size for the underlying `mpsc:channel`
    pub buffer_size: usize,

    /// The extension capabilities the sender supports
    pub offered_capabilities: Option<Vec<Symbol>>,

    /// The extension capabilities the sender can use if the receiver supports them
    pub desired_capabilities: Option<Vec<Symbol>>,

    /// Supported sender settle mode
    pub supported_snd_settle_modes: SupportedSenderSettleModes,

    /// The sender settle mode to fallback to when the mode desired
    /// by the remote peer is not supported.
    ///
    /// If this field is None, an incoming attach whose desired sender settle
    /// mode is not supported will then be rejected
    pub fallback_snd_settle_mode: SenderSettleMode,

    /// Supported receiver settle mode
    pub supported_rcv_settle_modes: SupportedReceiverSettleModes,

    /// The receiver settle mode to fallback to when the mode desired
    /// by the remote peer is not supported
    ///
    /// If this field is None, an incoming attach whose desired receiver settle
    /// mode is not supported will then be rejected
    pub fallback_rcv_settle_mode: ReceiverSettleMode,
}

impl Default for SharedLinkAcceptorFields {
    fn default() -> Self {
        Self {
            buffer_size: DEFAULT_OUTGOING_BUFFER_SIZE,
            max_message_size: None,
            properties: None,
            offered_capabilities: None,
            desired_capabilities: None,
            supported_snd_settle_modes: SupportedSenderSettleModes::default(),
            fallback_snd_settle_mode: SenderSettleMode::default(),
            supported_rcv_settle_modes: SupportedReceiverSettleModes::default(),
            fallback_rcv_settle_mode: ReceiverSettleMode::default(),
        }
    }
}

/// An acceptor for incoming links
///
/// # Accepts incoming link with default configuration
///
/// ```rust,ignore
/// use fe2o3_amqp::acceptor::{ListenerSessionHandle, LinkAcceptor, LinkEndpoint};
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
/// use fe2o3_amqp::acceptor::{LinkAcceptor, SupportedSenderSettleModes};
///
/// let link_acceptor = LinkAcceptor::builder()
///     .supported_sender_settle_modes(SupportedSenderSettleModes::Settled)
///     .build();
/// ```
///
#[derive(Debug, Clone)]
pub struct LinkAcceptor<FS, FT>
where
    FS: Fn(Source) -> Option<Source>,
    FT: Fn(Target) -> Option<Target>,
{
    pub(crate) shared: SharedLinkAcceptorFields,
    pub(crate) local_sender_acceptor: LocalSenderLinkAcceptor<Symbol, FS>,
    pub(crate) local_receiver_acceptor: LocalReceiverLinkAcceptor<Symbol, Target, FT>,
}

impl<FS, FT> std::fmt::Display for LinkAcceptor<FS, FT>
where
    FS: Fn(Source) -> Option<Source>,
    FT: Fn(Target) -> Option<Target>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("LinkAcceptor"))
    }
}

impl Default for LinkAcceptor<fn(Source) -> Option<Source>, fn(Target) -> Option<Target>> {
    fn default() -> Self {
        Self {
            shared: Default::default(),
            local_sender_acceptor: Default::default(),
            local_receiver_acceptor: Default::default(),
        }
    }
}

impl LinkAcceptor<fn(Source) -> Option<Source>, fn(Target) -> Option<Target>> {
    /// Creates a default LinkAcceptor
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a builder for [`LinkAcceptor`]
    pub fn builder() -> Builder<Self, Initialized> {
        Builder::<Self, Initialized>::new()
    }
}

impl<FS, FT> LinkAcceptor<FS, FT>
where
    FS: Fn(Source) -> Option<Source>,
    FT: Fn(Target) -> Option<Target>,
{
    /// Convert the acceptor into a link acceptor builder. This allows users to configure
    /// particular field using the builder pattern
    pub fn into_builder(self) -> Builder<Self, Initialized> {
        Builder {
            inner: self,
            marker: PhantomData,
        }
    }

    /// Accept incoming link with an explicit Attach performative
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
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
                    .map(LinkEndpoint::Receiver)
                    .map_err(Into::into)
            }
            Role::Receiver => self
                .local_sender_acceptor
                .accept_incoming_attach(&self.shared, remote_attach, session)
                .await
                .map(LinkEndpoint::Sender)
                .map_err(Into::into),
        }
    }

    /// Accept incoming link by waiting for an incoming Attach performative
    pub async fn accept(
        &self,
        session: &mut ListenerSessionHandle,
    ) -> Result<LinkEndpoint, AcceptorAttachError> {
        let remote_attach = match session.next_incoming_attach().await {
            Some(attach) => attach,
            None => {
                return Err(match session.session_stop_reason.get() {
                    Some(reason) => AcceptorAttachError::SessionStopped(reason.clone()),
                    None => {
                        // The session engine should always record a stop reason
                        // before its channels close; an unset cell here is a
                        // defensive fallback.
                        #[cfg(feature = "tracing")]
                        tracing::warn!(
                            "accept: session stop reason not recorded; reporting SessionStopped(Ended)"
                        );
                        #[cfg(feature = "log")]
                        log::warn!(
                            "accept: session stop reason not recorded; reporting SessionStopped(Ended)"
                        );
                        AcceptorAttachError::SessionStopped(SessionStopReason::Ended)
                    }
                });
            }
        };
        self.accept_incoming_attach(remote_attach, session).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, OnceLock};

    use fe2o3_amqp_types::performatives::Attach;
    use tokio::sync::{mpsc, oneshot};

    use super::{AcceptorAttachError, LinkAcceptor, ListenerSessionHandle, SessionHandle, SessionStopReason};
    use crate::{
        control::SessionControl,
        link::LinkFrame,
        session::error::Error,
    };

    /// Constructs a listener session handle in the "ended" state: the link
    /// listener sender is dropped (as if the session engine exited) and the
    /// stop reason cell is either pre-set or left unset.
    fn ended_listener_session_handle(
        session_stop_reason: Option<SessionStopReason>,
    ) -> ListenerSessionHandle {
        let (_, link_listener) = mpsc::channel::<Attach>(16);
        let (control, _) = mpsc::channel::<SessionControl>(16);
        let (outgoing, _) = mpsc::channel::<LinkFrame>(16);
        let (outcome_tx, outcome) = oneshot::channel::<Result<(), Error>>();
        drop(outcome_tx);
        let stop_reason_cell = Arc::new(OnceLock::new());
        if let Some(reason) = session_stop_reason {
            let _ = stop_reason_cell.set(reason);
        }
        SessionHandle {
            is_ended: false,
            control,
            engine_handle: tokio::spawn(async {}),
            outcome,
            outgoing,
            session_stop_reason: stop_reason_cell,
            link_listener,
        }
    }

    /// The recorded stop reason must be surfaced as-is when the link listener
    /// ends.
    #[tokio::test]
    async fn accept_reports_recorded_stop_reason() {
        let mut handle = ended_listener_session_handle(Some(SessionStopReason::ConnectionClosed));

        let result = LinkAcceptor::new().accept(&mut handle).await;

        match result {
            Err(AcceptorAttachError::SessionStopped(SessionStopReason::ConnectionClosed)) => {}
            other => panic!("expected SessionStopped(ConnectionClosed), got {:?}", other),
        }
    }

    /// When the link listener ends without a recorded stop reason (defensive
    /// fallback), the acceptor reports `Ended` rather than panicking.
    #[tokio::test]
    async fn accept_falls_back_to_ended_without_stop_reason() {
        let mut handle = ended_listener_session_handle(None);

        let result = LinkAcceptor::new().accept(&mut handle).await;

        match result {
            Err(AcceptorAttachError::SessionStopped(SessionStopReason::Ended)) => {}
            other => panic!("expected SessionStopped(Ended), got {:?}", other),
        }
    }
}
