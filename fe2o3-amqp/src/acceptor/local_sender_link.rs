//! Implements acceptor for a remote receiver link

use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use fe2o3_amqp_types::{
    definitions::{SenderSettleMode, SequenceNo},
    messaging::Target,
    performatives::Attach,
    primitives::Symbol,
};
use tokio::sync::{mpsc, Notify, RwLock};

use crate::{
    endpoint::{InputHandle, LinkAttach, LinkExt},
    link::{
        sender::SenderInner,
        state::{LinkFlowState, LinkFlowStateInner, LinkState},
        LinkRelay, SenderAttachError, SenderLink,
    },
    session::SessionHandle,
    util::{Consumer, Producer},
    Sender,
};

use super::{link::SharedLinkAcceptorFields, SupportedSenderSettleModes};

/// An acceptor for a remote receiver link
///
/// the sender is considered to hold the authoritative version of the
/// source properties, the receiver is considered to hold the authoritative version of the target properties.
#[derive(Debug, Clone)]
pub(crate) struct LocalSenderLinkAcceptor<C> {
    /// Supported sender settle mode
    pub supported_snd_settle_modes: SupportedSenderSettleModes,

    /// The sender settle mode to fallback to when the mode desired
    /// by the remote peer is not supported.
    ///
    /// If this field is None, an incoming attach whose desired sender settle
    /// mode is not supported will then be rejected
    pub fallback_snd_settle_mode: SenderSettleMode,

    /// This MUST NOT be null if role is sender,
    /// and it is ignored if the role is receiver.
    /// See subsection 2.6.7.
    pub initial_delivery_count: SequenceNo,

    /// the extension capabilities the sender supports/desires
    pub source_capabilities: Option<Vec<C>>,
}

impl<C> Default for LocalSenderLinkAcceptor<C>
where
    C: From<Symbol>,
{
    fn default() -> Self {
        Self {
            supported_snd_settle_modes: SupportedSenderSettleModes::default(),
            fallback_snd_settle_mode: SenderSettleMode::default(),
            initial_delivery_count: 0,
            source_capabilities: None,
        }
    }
}

impl LocalSenderLinkAcceptor<Symbol> {
    /// Accepts an incoming attach as a local sender
    pub async fn accept_incoming_attach<R>(
        &self,
        shared: &SharedLinkAcceptorFields,
        remote_attach: Attach,
        session: &mut SessionHandle<R>,
    ) -> Result<Sender, SenderAttachError> {
        let snd_settle_mode = if self
            .supported_snd_settle_modes
            .supports(&remote_attach.snd_settle_mode)
        {
            remote_attach.snd_settle_mode.clone()
        } else {
            self.fallback_snd_settle_mode.clone()
        };

        let (incoming_tx, mut incoming_rx) = mpsc::channel(shared.buffer_size);

        let flow_state_inner = LinkFlowStateInner {
            initial_delivery_count: self.initial_delivery_count,
            delivery_count: self.initial_delivery_count,
            link_credit: 0,
            available: 0,
            drain: false,
            properties: shared.properties.clone(),
        };
        let flow_state = Arc::new(LinkFlowState::sender(flow_state_inner));
        let notifier = Arc::new(Notify::new());
        let flow_state_producer = Producer::new(notifier.clone(), flow_state.clone());
        let flow_state_consumer = Consumer::new(notifier, flow_state);

        let unsettled = Arc::new(RwLock::new(BTreeMap::new()));
        // let state_code = Arc::new(AtomicU8::new(0));
        let link_handle = LinkRelay::Sender {
            tx: incoming_tx,
            output_handle: (),
            flow_state: flow_state_producer,
            unsettled: unsettled.clone(),
            receiver_settle_mode: remote_attach.rcv_settle_mode.clone(),
            // state_code: state_code.clone(),
        };

        // Allocate link in session
        let input_handle = InputHandle::from(remote_attach.handle.clone());
        let output_handle = super::session::allocate_incoming_link(
            &mut session.control,
            remote_attach.name.clone(),
            link_handle,
            input_handle,
        )
        .await?;

        // In this case, the sender is considered to hold the authoritative version of the
        // version of the source properties
        let local_source = remote_attach.source.clone().map(|s| {
            let mut source = *s;
            source.capabilities = self.source_capabilities.clone().map(Into::into);
            source
        });

        let mut link = SenderLink::<Target> {
            role: PhantomData,
            local_state: LinkState::Unattached, // will be set in `on_incoming_attach`

            name: remote_attach.name.clone(),
            output_handle: Some(output_handle),
            input_handle: None, // this will be set in `on_incoming_attach`
            snd_settle_mode,
            rcv_settle_mode: Default::default(), // Will take value from incoming attach
            source: local_source,
            target: None, // Will take value from incoming attach
            max_message_size: shared.max_message_size.unwrap_or_else(|| 0),
            offered_capabilities: shared.offered_capabilities.clone(),
            desired_capabilities: shared.desired_capabilities.clone(),
            flow_state: flow_state_consumer,
            unsettled,
        };

        let outgoing = session.outgoing.clone();

        match link.on_incoming_attach(remote_attach).await {
            Ok(_) => link.send_attach(&outgoing).await?,
            Err(attach_error) => {
                link.send_attach(&outgoing).await?;
                return Err(link
                    .handle_attach_error(
                        attach_error,
                        &outgoing,
                        &mut incoming_rx,
                        &session.control,
                    )
                    .await);
            }
        }

        let inner = SenderInner {
            link,
            buffer_size: shared.buffer_size,
            session: session.control.clone(),
            outgoing,
            incoming: incoming_rx,
        };
        Ok(Sender { inner })
    }
}
