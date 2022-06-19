//! Implements acceptor for a remote sender link

use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, ReceiverSettleMode},
    messaging::{DeliveryState, TargetArchetype},
    performatives::Attach,
    primitives::Symbol,
};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{
    control::SessionControl,
    endpoint::{InputHandle, LinkAttach, LinkAttachAcceptorExt},
    link::{
        self,
        receiver::{CreditMode, ReceiverInner},
        role,
        state::{LinkFlowState, LinkFlowStateInner, LinkState},
        target_archetype::TargetArchetypeExt,
        AttachError, LinkFrame, LinkIncomingItem, LinkRelay, ReceiverFlowState,
    },
    session::SessionHandle,
    Receiver,
};

use super::{link::SharedLinkAcceptorFields, SupportedReceiverSettleModes};

/// An acceptor for a remote Sender link
///
/// the sender is considered to hold the authoritative version of the
/// source properties, the receiver is considered to hold the authoritative version of the target properties.
#[derive(Debug, Clone)]
pub(crate) struct LocalReceiverLinkAcceptor<C> {
    /// Supported receiver settle mode
    pub supported_rcv_settle_modes: SupportedReceiverSettleModes,

    /// The receiver settle mode to fallback to when the mode desired
    /// by the remote peer is not supported
    ///
    /// If this field is None, an incoming attach whose desired receiver settle
    /// mode is not supported will then be rejected
    pub fallback_rcv_settle_mode: ReceiverSettleMode,

    /// Credit mode of the link. This has no effect on a sender
    pub credit_mode: CreditMode,

    /// the extension capabilities the sender supports/desires
    pub target_capabilities: Option<Vec<C>>,
}

impl<C> Default for LocalReceiverLinkAcceptor<C> {
    fn default() -> Self {
        Self {
            supported_rcv_settle_modes: SupportedReceiverSettleModes::default(),
            fallback_rcv_settle_mode: ReceiverSettleMode::default(),
            credit_mode: CreditMode::default(),
            target_capabilities: None,
        }
    }
}

impl LocalReceiverLinkAcceptor<Symbol> {
    pub async fn accept_incoming_attach<R>(
        &self,
        shared: &SharedLinkAcceptorFields,
        remote_attach: Attach,
        session: &mut SessionHandle<R>,
    ) -> Result<Receiver, (AttachError, Option<Attach>)> {
        self.accept_incoming_attach_inner(
            shared,
            remote_attach,
            &session.control,
            &session.outgoing,
        )
        .await
        .map(|inner| Receiver { inner })
    }
}

impl<C> LocalReceiverLinkAcceptor<C>
where
    C: Clone,
{
    pub async fn accept_incoming_attach_inner<T>(
        &self,
        shared: &SharedLinkAcceptorFields,
        remote_attach: Attach,
        control: &mpsc::Sender<SessionControl>,
        outgoing: &mpsc::Sender<LinkFrame>,
    ) -> Result<
        ReceiverInner<link::Link<role::Receiver, T, ReceiverFlowState, DeliveryState>>,
        (AttachError, Option<Attach>),
    >
    where
        T: Into<TargetArchetype>
            + TryFrom<TargetArchetype>
            + TargetArchetypeExt<Capability = C>
            + Clone
            + Send,
    {
        // The receiver SHOULD respect the sender’s desired settlement mode if
        // the sender initiates the attach exchange and the receiver supports the desired mode
        let rcv_settle_mode = if self
            .supported_rcv_settle_modes
            .supports(&remote_attach.rcv_settle_mode)
        {
            remote_attach.rcv_settle_mode.clone()
        } else {
            self.fallback_rcv_settle_mode.clone()
        };

        // Create channels for Session-Link communication
        let (incoming_tx, incoming_rx) = mpsc::channel::<LinkIncomingItem>(shared.buffer_size);

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
        let unsettled = Arc::new(RwLock::new(BTreeMap::new()));
        // let state_code = Arc::new(AtomicU8::new(0));
        let link_handle = LinkRelay::Receiver {
            tx: incoming_tx,
            output_handle: (),
            flow_state: flow_state_producer,
            unsettled: unsettled.clone(),
            receiver_settle_mode: rcv_settle_mode.clone(),
            // state_code: state_code.clone(),
            more: false,
        };

        // Allocate link in session
        let input_handle = InputHandle::from(remote_attach.handle.clone());
        let output_handle = super::session::allocate_incoming_link(
            control,
            remote_attach.name.clone(),
            link_handle,
            input_handle,
        )
        .await;
        let output_handle = match output_handle {
            Ok(handle) => handle,
            Err(err) => return Err((err.into(), Some(remote_attach))), // If allocation fails, should respond with an empty attach
        };

        let mut target = match &remote_attach.target {
            Some(t) => match T::try_from(*t.clone()) {
                Ok(t) => t,
                Err(_) => {
                    return Err((
                        AttachError::Local(definitions::Error::new(
                            AmqpError::NotImplemented,
                            "Coordinator is not implemented".to_string(),
                            None,
                        )),
                        Some(remote_attach),
                    ))
                }
            },
            None => return Err((AttachError::TargetIsNone, Some(remote_attach))),
        };

        // Set local link to the capabilities that are actually supported
        *target.capabilities_mut() = self.target_capabilities.clone().map(Into::into);

        let mut link = link::Link::<role::Receiver, T, ReceiverFlowState, DeliveryState> {
            role: PhantomData,
            local_state: LinkState::Unattached, // State change will be taken care of in `on_incoming_attach`
            // state_code,
            name: remote_attach.name.clone(),
            output_handle: Some(output_handle),
            input_handle: None, // will be set in `on_incoming_attach`
            snd_settle_mode: Default::default(), // Will take value from incoming attach
            rcv_settle_mode,
            source: None, // Will take value from incoming attach
            target: Some(target),
            max_message_size: shared.max_message_size.unwrap_or_else(|| 0),
            offered_capabilities: shared.offered_capabilities.clone(),
            desired_capabilities: shared.desired_capabilities.clone(),
            flow_state: flow_state_consumer,
            unsettled,
        };

        let mut outgoing = PollSender::new(outgoing.clone());
        link.on_incoming_attach_as_acceptor(remote_attach).await?;
        link.send_attach(&mut outgoing)
            .await
            .map_err(|err| (err.into(), None))?;

        let mut inner = ReceiverInner {
            link,
            buffer_size: shared.buffer_size,
            credit_mode: self.credit_mode.clone(),
            processed: 0,
            session: control.clone(),
            outgoing,
            incoming: ReceiverStream::new(incoming_rx),
            incomplete_transfer: None,
        };

        if let CreditMode::Auto(credit) = inner.credit_mode {
            tracing::debug!("Setting credits");
            inner
                .set_credit(credit)
                .await
                .map_err(|error| match AttachError::try_from(error) {
                    Ok(error) => (error, None),
                    Err(_) => unreachable!(),
                })?;
        }

        Ok(inner)
    }
}
