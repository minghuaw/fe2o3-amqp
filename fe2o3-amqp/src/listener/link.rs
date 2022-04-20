//! Link Listener

// /// Listener for incoming link
// #[derive(Debug)]
// pub struct LinkListener {}

use std::{marker::PhantomData, sync::Arc, collections::BTreeMap};

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{
        DeliveryNumber, DeliveryTag, Fields, MessageFormat, ReceiverSettleMode, Role,
        SenderSettleMode, SequenceNo, self,
    },
    messaging::DeliveryState,
    performatives::{Attach, Detach, Transfer},
    primitives::{Symbol, ULong},
};
use futures_util::{Future, Sink};
use tokio::sync::{RwLock, mpsc};
use tokio_util::sync::PollSender;

use crate::{
    endpoint::{self, Settlement, Link},
    link::{
        self, delivery::UnsettledMessage, receiver::CreditMode, role, state::{LinkFlowState, LinkFlowStateInner, LinkState},
        LinkFrame, ReceiverFlowState, SenderFlowState, AttachError, LinkHandle, LinkIncomingItem,
    },
    util::Initialized,
    Delivery, Payload, Receiver,
};

use super::{
    builder::Builder, session::ListenerSessionHandle, IncomingLink, SupportedReceiverSettleModes,
    SupportedSenderSettleModes,
};

///
#[derive(Debug)]
pub enum LinkEndpoint {
    /// Sender
    Sender,

    /// Receiver
    Receiver,
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
        session: &mut ListenerSessionHandle
    ) -> Result<(), AttachError> {
        let local_attach = match remote_attach.role {
            Role::Sender => {
                remote_attach.target = None;
                remote_attach
            },
            Role::Receiver => {
                remote_attach.source = None;
                remote_attach
            },
        };
        let frame = LinkFrame::Attach(local_attach);
        session.outgoing.send(frame).await
            .map_err(|_| AttachError::IllegalSessionState)?; // Session must have been dropped
        Ok(())
    }

    /// Accept incoming link with an explicit Attach performative
    pub async fn accept_incoming_attach(
        &self,
        remote_attach: Attach,
        session: &mut ListenerSessionHandle,
    ) -> Result<LinkEndpoint, AttachError> {
        // In this case, the sender is considered to hold the authoritative version of the
        // source properties, the receiver is considered to hold the authoritative version of the target properties.
        let link = match remote_attach.role {
            Role::Sender => {
                // Remote is sender -> local is receiver
                self.accept_as_new_receiver(remote_attach, session).await

                // let builder = Receiver::builder()
                //     .name(remote_attach.name);
            }
            Role::Receiver => {
                self.accept_as_new_sender(remote_attach, session).await
            }
        };
        todo!()
    }

    async fn accept_as_new_receiver(
        &self,
        remote_attach: Attach,
        session: &mut ListenerSessionHandle,
    ) -> Result<LinkEndpoint, AttachError> {
        // The receiver SHOULD respect the sender’s desired settlement mode if
        // the sender initiates the attach exchange and the receiver supports the desired mode
        if self.supported_rcv_settle_modes.supports(&remote_attach.rcv_settle_mode) {
            let buffer_size = self.buffer_size.clone();
            let credit_mode = self.credit_mode.clone();
            let (incoming_tx, incoming_rx) = mpsc::channel::<LinkIncomingItem>(self.buffer_size);
            let outgoing = PollSender::new(session.outgoing.clone());

            // let initial_delivery_count = match remote_attach.initial_delivery_count {
            //     Some(val) => val,
            //     None => {
            //         // This MUST NOT be null if role is sender, and it is ignored if the role is receiver.
            //         self.reject_incoming_attach(remote_attach, session).await?;
            //         return Err(AttachError::InitialDeliveryCountIsNull)
            //     }
            // };
            
            // Create shared flow state
            let flow_state_inner = LinkFlowStateInner {
                initial_delivery_count: 0, // This will be set in `on_incoming_attach`
                delivery_count: 0,
                link_credit: 0, // The link-credit and available variables are initialized to zero.
                available: 0,
                drain: false, // The drain flag is initialized to false.
                properties: None, // Will be set in `on_incoming_attach`
            };
            let flow_state = Arc::new(LinkFlowState::receiver(flow_state_inner));

            // TODO: 
            // let unsettled = match remote_attach.unsettled {
            //     Some(map) => Arc::new(RwLock::new(map)),
            //     None => Arc::new(RwLock::new(BTreeMap::new()))
            // };
            let unsettled = Arc::new(RwLock::new(BTreeMap::new()));
            let flow_state_producer = flow_state.clone();
            let flow_state_consumer = flow_state;
            let link_handle = LinkHandle::Receiver {
                tx: incoming_tx,
                flow_state: flow_state_producer,
                unsettled: unsettled.clone(),
                receiver_settle_mode: remote_attach.rcv_settle_mode.clone(),
                more: false,
            };

            // Allocate link in session
            let output_handle = 
                crate::session::allocate_link(&mut session.control, remote_attach.name.clone(), link_handle).await?;

            let mut link = link::Link::<role::Receiver, ReceiverFlowState, DeliveryState> {
                role: PhantomData,
                local_state: LinkState::Unattached, // State change will be taken care of in `on_incoming_attach`
                name: remote_attach.name.clone(),
                output_handle: Some(output_handle),
                input_handle: None, // will be set in `on_incoming_attach`
                snd_settle_mode: remote_attach.snd_settle_mode.clone(),
                rcv_settle_mode: remote_attach.rcv_settle_mode.clone(),
                source: remote_attach.source.clone(),
                target: remote_attach.target.clone(),
                max_message_size: self.max_message_size.unwrap_or_else(|| 0),
                offered_capabilities: self.offered_capabilities.clone(),
                desired_capabilities: self.desired_capabilities.clone(),
                flow_state: flow_state_consumer,
                unsettled,
            };

            link.on_incoming_attach(remote_attach).await?;

        }

        todo!()
    }

    async fn accept_as_new_sender(
        &self,
        remote_attach: Attach,
        session: &mut ListenerSessionHandle,
    ) -> Result<LinkEndpoint, AttachError> {
        todo!()
    }

    // /// An alias for [`accept_with_incoming_attach`] for consistency
    // pub async fn accept_incoming_link(
    //     &self,
    //     incoming_link: IncomingLink,
    //     session: &mut ListenerSessionHandle,
    // ) -> Result<LinkEndpoint, link::Error> {
    //     todo!()
    // }

    /// Accept incoming link by waiting for an incoming Attach performative
    pub async fn accept(
        &self,
        session: &mut ListenerSessionHandle,
    ) -> Result<LinkEndpoint, link::Error> {
        todo!()
    }
}

// /// A link on the listener side
// #[derive(Debug)]
// pub struct ListenerLink<R, F, M> {
//     link: crate::link::Link<R, F, M>,
// }

// #[async_trait]
// impl<R, F, M> endpoint::Link for ListenerLink<R, F, M>
// where
//     R: role::IntoRole + Send + Sync,
//     F: AsRef<LinkFlowState<R>> + Send + Sync,
//     M: AsRef<DeliveryState> + AsMut<DeliveryState> + Send + Sync,
// {
//     type DetachError = <link::Link<R, F, M> as endpoint::Link>::DetachError;

//     type Error = <link::Link<R, F, M> as endpoint::Link>::Error;

//     async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error> {
//         todo!()
//     }

//     async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::DetachError> {
//         todo!()
//     }

//     async fn send_attach<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
//     where
//         W: Sink<LinkFrame> + Send + Unpin,
//     {
//         todo!()
//     }

//     async fn send_detach<W>(
//         &mut self,
//         writer: &mut W,
//         closed: bool,
//         error: Option<Self::DetachError>,
//     ) -> Result<(), Self::Error>
//     where
//         W: Sink<LinkFrame> + Send + Unpin,
//     {
//         todo!()
//     }
// }

// type ListenerSenderLink = ListenerLink<role::Sender, SenderFlowState, UnsettledMessage>;
// type ListenerReceiverLink = ListenerLink<role::Receiver, ReceiverFlowState, DeliveryState>;

// #[async_trait]
// impl endpoint::SenderLink for ListenerSenderLink {
// /// Set and send flow state
//     async fn send_flow<W>(
//         &mut self,
//         writer: &mut W,
//         delivery_count: Option<SequenceNo>,
//         available: Option<u32>,
//         echo: bool,
//     ) -> Result<(), Self::Error>
//     where
//         W: Sink<LinkFrame> + Send + Unpin { todo!() }

//     /// Send message via transfer frame and return whether the message is already settled
//     async fn send_transfer<W, Fut>(
//         &mut self,
//         writer: &mut W,
//         detached: Fut,
//         payload: Payload,
//         message_format: MessageFormat,
//         settled: Option<bool>,
//         batchable: bool,
//     ) -> Result<Settlement, <Self as endpoint::Link>::Error>
//     where
//         W: Sink<LinkFrame> + Send + Unpin,
//         Fut: Future<Output = Option<LinkFrame>> + Send { todo!() }

//     async fn dispose<W>(
//         &mut self,
//         writer: &mut W,
//         delivery_id: DeliveryNumber,
//         delivery_tag: DeliveryTag,
//         settled: bool,
//         state: DeliveryState,
//         batchable: bool,
//     ) -> Result<(), Self::Error>
//     where
//         W: Sink<LinkFrame> + Send + Unpin { todo!() }

//     async fn batch_dispose<W>(
//         &mut self,
//         writer: &mut W,
//         ids_and_tags: Vec<(DeliveryNumber, DeliveryTag)>,
//         settled: bool,
//         state: DeliveryState,
//         batchable: bool,
//     ) -> Result<(), Self::Error>
//     where
//         W: Sink<LinkFrame> + Send + Unpin { todo!() }
// }

// #[async_trait]
// impl endpoint::ReceiverLink for ListenerReceiverLink {
//     /// Set and send flow state
//     async fn send_flow<W>(
//         &mut self,
//         writer: &mut W,
//         link_credit: Option<u32>,
//         drain: Option<bool>, // TODO: Is Option necessary?
//         echo: bool,
//     ) -> Result<(), Self::Error>
//     where
//         W: Sink<LinkFrame> + Send + Unpin { todo!() }

//     async fn on_incomplete_transfer(
//         &mut self,
//         delivery_tag: DeliveryTag,
//         section_number: u32,
//         section_offset: u64,
//     ) { todo!() }

//     // More than one transfer frames should be hanlded by the
//     // `Receiver`
//     async fn on_incoming_transfer<T>(
//         &mut self,
//         transfer: Transfer,
//         payload: Payload,
//     ) -> Result<
//         (
//             Delivery<T>,
//             Option<(DeliveryNumber, DeliveryTag, DeliveryState)>,
//         ),
//         <Self as endpoint::Link>::Error,
//     >
//     where
//         T: for<'de> serde::Deserialize<'de> + Send { todo!() }

//     async fn dispose<W>(
//         &mut self,
//         writer: &mut W,
//         delivery_id: DeliveryNumber,
//         delivery_tag: DeliveryTag,
//         // settled: bool, // TODO: This should depend on ReceiverSettleMode?
//         state: DeliveryState,
//         batchable: bool,
//     ) -> Result<(), Self::Error>
//     where
//         W: Sink<LinkFrame> + Send + Unpin { todo!() }
// }
