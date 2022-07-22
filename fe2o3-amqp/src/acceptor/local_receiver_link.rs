//! Implements acceptor for a remote sender link

use std::{marker::PhantomData, sync::Arc};

use fe2o3_amqp_types::{
    messaging::TargetArchetype, performatives::Attach,
    primitives::Symbol,
};
use tokio::sync::{mpsc, RwLock};
use tracing::instrument;

use crate::{
    control::SessionControl,
    endpoint::{InputHandle, LinkAttach, LinkExt},
    link::{
        receiver::{CreditMode, ReceiverInner},
        state::{LinkFlowState, LinkFlowStateInner, LinkState},
        target_archetype::TargetArchetypeExt,
        LinkFrame, LinkIncomingItem, LinkRelay, ReceiverAttachError, ReceiverLink,
    },
    session::SessionHandle,
    Receiver,
};

use super::{link::SharedLinkAcceptorFields};

/// An acceptor for a remote Sender link
///
/// the sender is considered to hold the authoritative version of the
/// source properties, the receiver is considered to hold the authoritative version of the target properties.
#[derive(Debug, Clone)]
pub(crate) struct LocalReceiverLinkAcceptor<C> {
    // /// Supported receiver settle mode
    // pub supported_rcv_settle_modes: SupportedReceiverSettleModes,

    // /// The receiver settle mode to fallback to when the mode desired
    // /// by the remote peer is not supported
    // ///
    // /// If this field is None, an incoming attach whose desired receiver settle
    // /// mode is not supported will then be rejected
    // pub fallback_rcv_settle_mode: ReceiverSettleMode,

    /// Credit mode of the link. This has no effect on a sender
    pub credit_mode: CreditMode,

    /// the extension capabilities the sender supports/desires
    pub target_capabilities: Option<Vec<C>>,

    /// Whether the receiver will automatically accept all incoming deliveries
    /// # Default
    ///
    /// ```rust
    /// auto_accept = false;
    /// ```
    pub auto_accept: bool,
}

impl<C> Default for LocalReceiverLinkAcceptor<C> {
    fn default() -> Self {
        Self {
            // supported_rcv_settle_modes: SupportedReceiverSettleModes::default(),
            // fallback_rcv_settle_mode: ReceiverSettleMode::default(),
            credit_mode: CreditMode::default(),
            target_capabilities: None,
            auto_accept: false,
        }
    }
}

impl LocalReceiverLinkAcceptor<Symbol> {
    pub async fn accept_incoming_attach<R>(
        &self,
        shared: &SharedLinkAcceptorFields,
        remote_attach: Attach,
        session: &mut SessionHandle<R>,
    ) -> Result<Receiver, ReceiverAttachError> {
        self.accept_incoming_attach_inner(
            shared,
            remote_attach,
            session.control.clone(),
            session.outgoing.clone(),
        )
        .await
        .map(|inner| Receiver { inner })
    }
}

impl<C> LocalReceiverLinkAcceptor<C>
where
    C: Clone,
{
    #[instrument(skip_all)]
    pub async fn accept_incoming_attach_inner<T>(
        &self,
        shared: &SharedLinkAcceptorFields,
        remote_attach: Attach,
        control: mpsc::Sender<SessionControl>,
        outgoing: mpsc::Sender<LinkFrame>,
    ) -> Result<ReceiverInner<ReceiverLink<T>>, ReceiverAttachError>
    where
        T: Into<TargetArchetype>
            + TryFrom<TargetArchetype>
            + TargetArchetypeExt<Capability = C>
            + Clone
            + Send
            + Sync,
    {
        let snd_settle_mode = if shared
            .supported_snd_settle_modes
            .supports(&remote_attach.snd_settle_mode)
        {
            remote_attach.snd_settle_mode.clone()
        } else {
            shared.fallback_snd_settle_mode.clone()
        };
        // The receiver SHOULD respect the sender’s desired settlement mode if
        // the sender initiates the attach exchange and the receiver supports the desired mode
        let rcv_settle_mode = if shared
            .supported_rcv_settle_modes
            .supports(&remote_attach.rcv_settle_mode)
        {
            remote_attach.rcv_settle_mode.clone()
        } else {
            shared.fallback_rcv_settle_mode.clone()
        };

        // Create channels for Session-Link communication
        let (incoming_tx, mut incoming_rx) = mpsc::channel::<LinkIncomingItem>(shared.buffer_size);

        // Create shared flow state
        let flow_state_inner = LinkFlowStateInner {
            initial_delivery_count: 0, // This will be set in `on_incoming_attach`
            delivery_count: 0,
            link_credit: 0, // The link-credit and available variables are initialized to zero.
            available: 0,
            drain: false, // The drain flag is initialized to false.
            properties: shared.properties.clone(), // Will be set in `on_incoming_attach`
        };
        let flow_state = Arc::new(LinkFlowState::receiver(flow_state_inner));
        let flow_state_producer = flow_state.clone();
        let flow_state_consumer = flow_state;

        // Comparing unsettled should be taken care of in `on_incoming_attach`
        let unsettled = Arc::new(RwLock::new(None));
        let link_handle = LinkRelay::Receiver {
            tx: incoming_tx,
            output_handle: (),
            flow_state: flow_state_producer,
            unsettled: unsettled.clone(),
            receiver_settle_mode: rcv_settle_mode.clone(),
            more: false,
        };

        // Allocate link in session
        let input_handle = InputHandle::from(remote_attach.handle.clone());
        let output_handle = super::session::allocate_incoming_link(
            &control,
            remote_attach.name.clone(),
            link_handle,
            input_handle,
        )
        .await?;

        let mut err = None;
        // **the receiver is considered to hold the authoritative version of the target properties**,
        let local_target = remote_attach
            .target
            .clone()
            .map(|t| T::try_from(*t))
            .transpose()
            .map(|mut t| {
                if let Some(target) = t.as_mut() {
                    *target.capabilities_mut() = self.target_capabilities.clone().map(Into::into);
                }
                t
            })
            .unwrap_or_else(|_| {
                err = Some(ReceiverAttachError::CoordinatorIsNotImplemented);
                None
            });

        let mut link = ReceiverLink::<T> {
            role: PhantomData,
            local_state: LinkState::Unattached, // State change will be taken care of in `on_incoming_attach`
            name: remote_attach.name.clone(),
            output_handle: Some(output_handle),
            input_handle: None, // will be set in `on_incoming_attach`
            snd_settle_mode,
            rcv_settle_mode,
            source: None,         // Will take value from incoming attach
            target: local_target, // Will take value from incoming attach
            max_message_size: shared.max_message_size.unwrap_or(0),
            offered_capabilities: shared.offered_capabilities.clone(),
            desired_capabilities: shared.desired_capabilities.clone(),
            flow_state: flow_state_consumer,
            unsettled,
        };

        match (err, link.on_incoming_attach(remote_attach).await) {
            (Some(attach_error), _) | (_, Err(attach_error)) => {
                // Complete attach anyway
                link.send_attach(&outgoing, &control, false).await?;
                match attach_error {
                    ReceiverAttachError::SndSettleModeNotSupported | ReceiverAttachError::RcvSettleModeNotSupported => {
                        // FIXME: Ths initiating end should be responsible for checking whether the mode is supported
                    },
                    _ => return Err(link
                        .handle_attach_error(attach_error, &outgoing, &mut incoming_rx, &control)
                        .await)
                }
            }
            (_, Ok(_)) => link.send_attach(&outgoing, &control, false).await?,
        }

        let mut inner = ReceiverInner {
            link,
            buffer_size: shared.buffer_size,
            credit_mode: self.credit_mode.clone(),
            processed: 0,
            auto_accept: self.auto_accept,
            session: control.clone(),
            outgoing,
            incoming: incoming_rx,
            incomplete_transfer: None,
        };

        if let CreditMode::Auto(credit) = inner.credit_mode {
            tracing::debug!("Setting credits");
            inner.set_credit(credit).await?;
        }

        Ok(inner)
    }
}
