//! Link Listener

// /// Listener for incoming link
// #[derive(Debug)]
// pub struct LinkListener {}

use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use fe2o3_amqp_types::{
    definitions::{Fields, ReceiverSettleMode, Role, SenderSettleMode, SequenceNo},
    messaging::DeliveryState,
    performatives::Attach,
    primitives::{Symbol, ULong},
};
use tokio::sync::{mpsc, Notify, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{
    endpoint::Link,
    link::{
        self,
        delivery::UnsettledMessage,
        receiver::CreditMode,
        role,
        state::{LinkFlowState, LinkFlowStateInner, LinkState},
        type_state::Attached,
        AttachError, LinkFrame, LinkHandle, LinkIncomingItem, ReceiverFlowState, SenderFlowState,
    },
    util::{Consumer, Initialized, Producer},
    Receiver, Sender,
};

use super::{
    builder::Builder, session::ListenerSessionHandle, SupportedReceiverSettleModes,
    SupportedSenderSettleModes,
};

/// Listener side link endpoint
#[derive(Debug)]
pub enum LinkEndpoint {
    /// Sender
    Sender(crate::link::Sender),

    /// Receiver
    Receiver(crate::link::Receiver<Attached>),
}

/// An acceptor for incoming links
#[derive(Debug)]
pub struct LinkAcceptor {
    /// Supported sender settle mode
    pub supported_snd_settle_modes: SupportedSenderSettleModes,

    /// The sender settle mode to fallback to when the mode desired
    /// by the remote peer is not supported.
    ///
    /// If this field is None, an incoming attach whose desired sender settle
    /// mode is not supported will then be rejected
    pub fallback_snd_settle_mode: Option<SenderSettleMode>,

    /// Supported receiver settle mode
    pub supported_rcv_settle_modes: SupportedReceiverSettleModes,

    /// The receiver settle mode to fallback to when the mode desired
    /// by the remote peer is not supported
    ///
    /// If this field is None, an incoming attach whose desired receiver settle
    /// mode is not supported will then be rejected
    pub fallback_rcv_settle_mode: Option<ReceiverSettleMode>,

    /// This MUST NOT be null if role is sender,
    /// and it is ignored if the role is receiver.
    /// See subsection 2.6.7.
    pub initial_delivery_count: SequenceNo,

    /// The maximum message size supported by the link endpoint
    pub max_message_size: Option<ULong>,

    /// The extension capabilities the sender supports
    pub offered_capabilities: Option<Vec<Symbol>>,

    /// The extension capabilities the sender can use if the receiver supports them
    pub desired_capabilities: Option<Vec<Symbol>>,

    /// Link properties
    pub properties: Option<Fields>,

    /// Buffer size for the underlying `mpsc:channel`
    pub buffer_size: usize,

    /// Credit mode of the link. This has no effect on a sender
    pub credit_mode: CreditMode,
}

impl std::fmt::Display for LinkAcceptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("LinkAcceptor"))
    }
}

impl LinkAcceptor {
    /// Creates a default LinkAcceptor
    pub fn new() -> Self {
        Self::builder().build()
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

    /// Reject an incoming attach with an attach that has either target
    /// or source field left empty (None or Null)
    pub async fn reject_incoming_attach(
        &self,
        mut remote_attach: Attach,
        session: &mut ListenerSessionHandle,
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
        session
            .outgoing
            .send(frame)
            .await
            .map_err(|_| AttachError::IllegalSessionState)?; // Session must have been dropped
        Ok(())
    }

    /// Accept incoming link with an explicit Attach performative
    pub async fn accept_incoming_attach(
        &self,
        remote_attach: Attach,
        session: &mut ListenerSessionHandle,
    ) -> Result<LinkEndpoint, AttachError> {
        match (
            &remote_attach.source.is_none(),
            &remote_attach.target.is_none(),
        ) {
            (true, _) => {
                self.reject_incoming_attach(remote_attach, session).await?;
                return Err(AttachError::SourceIsNone);
            }
            (_, true) => {
                self.reject_incoming_attach(remote_attach, session).await?;
                return Err(AttachError::TargetIsNone);
            }
            _ => {}
        }

        // In this case, the sender is considered to hold the authoritative version of the
        // source properties, the receiver is considered to hold the authoritative version of the target properties.
        match remote_attach.role {
            Role::Sender => {
                // Remote is sender -> local is receiver
                self.accept_as_new_receiver(remote_attach, session).await
            }
            Role::Receiver => self.accept_as_new_sender(remote_attach, session).await,
        }
    }

    async fn accept_as_new_receiver(
        &self,
        remote_attach: Attach,
        session: &mut ListenerSessionHandle,
    ) -> Result<LinkEndpoint, AttachError> {
        // The receiver SHOULD respect the sender’s desired settlement mode if
        // the sender initiates the attach exchange and the receiver supports the desired mode
        let rcv_settle_mode = if self
            .supported_rcv_settle_modes
            .supports(&remote_attach.rcv_settle_mode)
        {
            remote_attach.rcv_settle_mode.clone()
        } else {
            self.fallback_rcv_settle_mode
                .clone()
                .ok_or_else(|| AttachError::ReceiverSettleModeNotSupported)?
        };

        // Create channels for Session-Link communication
        let (incoming_tx, incoming_rx) = mpsc::channel::<LinkIncomingItem>(self.buffer_size);

        // Create shared flow state
        let flow_state_inner = LinkFlowStateInner {
            initial_delivery_count: 0, // This will be set in `on_incoming_attach`
            delivery_count: 0,
            link_credit: 0, // The link-credit and available variables are initialized to zero.
            available: 0,
            drain: false, // The drain flag is initialized to false.
            properties: self.properties.clone(), // Will be set in `on_incoming_attach`
        };
        let flow_state = Arc::new(LinkFlowState::receiver(flow_state_inner));
        let flow_state_producer = flow_state.clone();
        let flow_state_consumer = flow_state;

        // Comparing unsettled should be taken care of in `on_incoming_attach`
        let unsettled = Arc::new(RwLock::new(BTreeMap::new()));
        let link_handle = LinkHandle::Receiver {
            tx: incoming_tx,
            flow_state: flow_state_producer,
            unsettled: unsettled.clone(),
            receiver_settle_mode: rcv_settle_mode.clone(),
            more: false,
        };

        // Allocate link in session
        let input_handle = remote_attach.handle.clone();
        let output_handle = crate::session::allocate_incoming_link(
            &mut session.control,
            remote_attach.name.clone(),
            link_handle,
            input_handle,
        )
        .await?;

        let mut link = link::Link::<role::Receiver, ReceiverFlowState, DeliveryState> {
            role: PhantomData,
            local_state: LinkState::Unattached, // State change will be taken care of in `on_incoming_attach`
            name: remote_attach.name.clone(),
            output_handle: Some(output_handle),
            input_handle: None, // will be set in `on_incoming_attach`
            snd_settle_mode: remote_attach.snd_settle_mode.clone(),
            rcv_settle_mode,
            source: remote_attach.source.clone(),
            target: remote_attach.target.clone(),
            max_message_size: self.max_message_size.unwrap_or_else(|| 0),
            offered_capabilities: self.offered_capabilities.clone(),
            desired_capabilities: self.desired_capabilities.clone(),
            flow_state: flow_state_consumer,
            unsettled,
        };

        let mut outgoing = PollSender::new(session.outgoing.clone());
        link.on_incoming_attach(remote_attach).await?;
        link.send_attach(&mut outgoing).await?;

        let mut receiver = Receiver::<Attached> {
            link,
            buffer_size: self.buffer_size,
            credit_mode: self.credit_mode.clone(),
            processed: 0,
            session: session.control.clone(),
            outgoing,
            incoming: ReceiverStream::new(incoming_rx),
            marker: PhantomData,
            incomplete_transfer: None,
        };

        if let CreditMode::Auto(credit) = receiver.credit_mode {
            tracing::debug!("Setting credits");
            receiver.set_credit(credit).await.map_err(|error| {
                match AttachError::try_from(error) {
                    Ok(error) => error,
                    Err(_) => unreachable!(),
                }
            })?;
        }

        Ok(LinkEndpoint::Receiver(receiver))
    }

    async fn accept_as_new_sender(
        &self,
        remote_attach: Attach,
        session: &mut ListenerSessionHandle,
    ) -> Result<LinkEndpoint, AttachError> {
        let snd_settle_mode = if self
            .supported_snd_settle_modes
            .supports(&remote_attach.snd_settle_mode)
        {
            remote_attach.snd_settle_mode.clone()
        } else {
            self.fallback_snd_settle_mode
                .clone()
                .ok_or_else(|| AttachError::SenderSettleModeNotSupported)?
        };

        let (incoming_tx, incoming_rx) = mpsc::channel(self.buffer_size);

        let flow_state_inner = LinkFlowStateInner {
            initial_delivery_count: self.initial_delivery_count,
            delivery_count: self.initial_delivery_count,
            link_credit: 0,
            available: 0,
            drain: false,
            properties: self.properties.clone(),
        };
        let flow_state = Arc::new(LinkFlowState::sender(flow_state_inner));
        let notifier = Arc::new(Notify::new());
        let flow_state_producer = Producer::new(notifier.clone(), flow_state.clone());
        let flow_state_consumer = Consumer::new(notifier, flow_state);

        let unsettled = Arc::new(RwLock::new(BTreeMap::new()));
        let link_handle = LinkHandle::Sender {
            tx: incoming_tx,
            flow_state: flow_state_producer,
            unsettled: unsettled.clone(),
            receiver_settle_mode: remote_attach.rcv_settle_mode.clone(),
        };

        // Allocate link in session
        let input_handle = remote_attach.handle.clone();
        let output_handle = crate::session::allocate_incoming_link(
            &mut session.control,
            remote_attach.name.clone(),
            link_handle,
            input_handle,
        )
        .await?;

        let mut link = link::Link::<role::Sender, SenderFlowState, UnsettledMessage> {
            role: PhantomData,
            local_state: LinkState::Unattached, // will be set in `on_incoming_attach`
            name: remote_attach.name.clone(),
            output_handle: Some(output_handle),
            input_handle: None, // this will be set in `on_incoming_attach`
            snd_settle_mode,
            rcv_settle_mode: remote_attach.rcv_settle_mode.clone(),
            source: remote_attach.source.clone(),
            target: remote_attach.target.clone(),
            max_message_size: self.max_message_size.unwrap_or_else(|| 0),
            offered_capabilities: self.offered_capabilities.clone(),
            desired_capabilities: self.desired_capabilities.clone(),
            flow_state: flow_state_consumer,
            unsettled,
        };

        let mut outgoing = PollSender::new(session.outgoing.clone());
        link.on_incoming_attach(remote_attach).await?;
        link.send_attach(&mut outgoing).await?;

        let sender = Sender {
            link,
            buffer_size: self.buffer_size,
            session: session.control.clone(),
            outgoing,
            incoming: ReceiverStream::new(incoming_rx),
            // marker: PhantomData,
        };
        Ok(LinkEndpoint::Sender(sender))
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
