//! Trait abstraction of Connection, Session and Link
//!
//! Frame          Connection  Session  Link
//! ========================================
//! open               H
//! begin              I          H
//! attach                        I       H
//! flow                          I       H
//! transfer                      I       H
//! disposition                   I       H
//! detach                        I       H
//! end                I          H
//! close              H
//! ----------------------------------------
//! Key:
//!     H: handled by the endpoint
//!     I: intercepted (endpoint examines
//!         the frame, but delegates
//!         further processing to another
//!         endpoint)

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{
        DeliveryNumber, DeliveryTag, Error, Fields, Handle, MessageFormat, ReceiverSettleMode,
        Role, SequenceNo,
    },
    messaging::DeliveryState,
    performatives::{Attach, Begin, Close, Detach, Disposition, End, Flow, Open, Transfer},
    primitives::{Boolean, UInt},
};
use futures_util::{Future, Sink};
use tokio::sync::{mpsc, oneshot};

use crate::{
    frames::amqp::Frame,
    link::{delivery::Delivery, LinkFrame},
    session::frame::{SessionFrame, SessionIncomingItem},
    Payload,
};

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct OutgoingChannel(pub u16);

impl From<OutgoingChannel> for u16 {
    fn from(channel: OutgoingChannel) -> Self {
        channel.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub(crate) struct IncomingChannel(pub u16);

impl From<IncomingChannel> for u16 {
    fn from(channel: IncomingChannel) -> Self {
        channel.0
    }
}

#[async_trait]
pub(crate) trait Connection {
    type AllocError: Send;
    type Error: Send;
    type State: Send;
    type Session: Session + Send;

    fn local_state(&self) -> &Self::State;
    fn local_state_mut(&mut self) -> &mut Self::State;
    fn local_open(&self) -> &Open;

    // Allocate outgoing channel id and session id to a new session
    fn allocate_session(
        &mut self,
        tx: mpsc::Sender<SessionIncomingItem>,
    ) -> Result<OutgoingChannel, Self::AllocError>;
    // Remove outgoing id and session id association
    fn deallocate_session(&mut self, outgoing_channel: OutgoingChannel);

    // async fn forward_to_session(&mut self, incoming_channel: u16, frame: SessionFrame) -> Result<(), Self::Error>;

    /// Reacting to remote Open frame
    async fn on_incoming_open(&mut self, channel: IncomingChannel, open: Open) -> Result<(), Self::Error>;

    /// Reacting to remote Begin frame
    ///
    /// Do NOT forward to session here. Forwarding is handled elsewhere.
    async fn on_incoming_begin(&mut self, channel: IncomingChannel, begin: Begin) -> Result<(), Self::Error>;

    /// Reacting to remote End frame
    async fn on_incoming_end(&mut self, channel: IncomingChannel, end: End) -> Result<(), Self::Error>;

    /// Reacting to remote Close frame
    async fn on_incoming_close(&mut self, channel: IncomingChannel, close: Close) -> Result<(), Self::Error>;

    /// Sending out an Open frame
    ///
    /// The write is passed in is because sending an Open frame also changes the local
    /// connection state. If the sending fails outside, coming back
    /// and revert the state changes would be too complicated
    async fn send_open<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<Frame> + Send + Unpin,
        W::Error: Into<Self::Error>; // DO NOT remove this. This is where `Transport` will be used

    async fn send_close<W>(
        &mut self,
        writer: &mut W,
        error: Option<Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<Frame> + Send + Unpin,
        W::Error: Into<Self::Error>; // DO NOT remove this. This is where `Transport` will be used

    /// Intercepting session frames
    fn on_outgoing_begin(&mut self, channel: OutgoingChannel, begin: Begin) -> Result<Frame, Self::Error>;

    fn on_outgoing_end(&mut self, channel: OutgoingChannel, end: End) -> Result<Frame, Self::Error>;

    fn session_tx_by_incoming_channel(
        &mut self,
        incoming_channel: IncomingChannel,
    ) -> Option<&mpsc::Sender<SessionIncomingItem>>;

    fn session_tx_by_outgoing_channel(
        &mut self,
        outgoing_channel: OutgoingChannel,
    ) -> Option<&mpsc::Sender<SessionIncomingItem>>;
}

#[async_trait]
pub(crate) trait Session {
    type AllocError: Send;
    type Error: Send;
    type State;
    type LinkRelay;

    fn local_state(&self) -> &Self::State;
    fn local_state_mut(&mut self) -> &mut Self::State;
    fn outgoing_channel(&self) -> OutgoingChannel;

    // Allocate new local handle for new Link
    fn allocate_link(
        &mut self,
        link_name: String,
        link_handle: Self::LinkRelay,
    ) -> Result<Handle, Self::AllocError>;
    fn allocate_incoming_link(
        &mut self,
        link_name: String,
        link_handle: Self::LinkRelay,
        input_handle: Handle,
    ) -> Result<Handle, Self::AllocError>;
    fn deallocate_link(&mut self, link_name: String);

    fn on_incoming_begin(&mut self, channel: u16, begin: Begin) -> Result<(), Self::Error>;
    async fn on_incoming_attach(&mut self, channel: u16, attach: Attach)
        -> Result<(), Self::Error>;
    async fn on_incoming_flow(&mut self, channel: u16, flow: Flow) -> Result<(), Self::Error>;
    async fn on_incoming_transfer(
        &mut self,
        channel: u16,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<(), Self::Error>;
    async fn on_incoming_disposition(
        &mut self,
        channel: u16,
        disposition: Disposition,
    ) -> Result<(), Self::Error>;
    async fn on_incoming_detach(&mut self, channel: u16, detach: Detach)
        -> Result<(), Self::Error>;
    async fn on_incoming_end(&mut self, channel: u16, end: End) -> Result<(), Self::Error>;

    // Handling SessionFrames
    async fn send_begin<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame> + Send + Unpin;

    async fn send_end<W>(
        &mut self,
        writer: &mut W,
        error: Option<Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame> + Send + Unpin;

    // Intercepting LinkFrames
    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error>;
    fn on_outgoing_flow(&mut self, flow: LinkFlow) -> Result<SessionFrame, Self::Error>;
    fn on_outgoing_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<SessionFrame, Self::Error>;
    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error>;
    fn on_outgoing_detach(&mut self, detach: Detach) -> Result<SessionFrame, Self::Error>;
}

#[async_trait]
pub(crate) trait LinkDetach {
    type DetachError: Send;

    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::DetachError>;

    async fn send_detach<W>(
        &mut self,
        writer: &mut W,
        closed: bool,
        error: Option<Self::DetachError>,
    ) -> Result<(), Self::DetachError>
    where
        W: Sink<LinkFrame> + Send + Unpin;
}

#[async_trait]
pub(crate) trait LinkAttach {
    type AttachError: Send;

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::AttachError>;

    async fn send_attach<W>(&mut self, writer: &mut W) -> Result<(), Self::AttachError>
    where
        W: Sink<LinkFrame> + Send + Unpin;
}

pub(crate) trait Link: LinkAttach + LinkDetach {}

pub(crate) trait LinkExt: Link {
    type FlowState;
    type Unsettled;

    fn name(&self) -> &str;

    fn output_handle(&self) -> &Option<Handle>;

    fn output_handle_mut(&mut self) -> &mut Option<Handle>;

    fn flow_state(&self) -> &Self::FlowState;

    fn unsettled(&self) -> &Self::Unsettled;

    fn rcv_settle_mode(&self) -> &ReceiverSettleMode;
}

/// A subset of the fields in the Flow performative
#[derive(Debug, Default)]
pub struct LinkFlow {
    /// Link handle
    pub handle: Handle,

    /// The endpoint’s value for the delivery-count sequence number
    pub delivery_count: Option<SequenceNo>,

    /// The current maximum number of messages that can be received
    pub link_credit: Option<UInt>,

    /// The number of available messages
    pub available: Option<UInt>,

    /// Indicates drain mode
    pub drain: Boolean,

    /// Request state from partner
    pub echo: Boolean,

    /// Link state properties
    pub properties: Option<Fields>,
}

impl TryFrom<Flow> for LinkFlow {
    type Error = ();

    fn try_from(value: Flow) -> Result<Self, Self::Error> {
        let flow = LinkFlow {
            handle: value.handle.ok_or(())?,
            delivery_count: value.delivery_count,
            link_credit: value.link_credit,
            available: value.available,
            drain: value.drain,
            echo: value.echo,
            properties: value.properties,
        };
        Ok(flow)
    }
}

pub(crate) enum Settlement {
    Settled,
    Unsettled {
        _delivery_tag: [u8; 4],
        outcome: oneshot::Receiver<DeliveryState>,
    },
}

#[async_trait]
pub(crate) trait SenderLink: Link + LinkExt {
    type Error: Send;

    const ROLE: Role = Role::Sender;

    /// Set and send flow state
    async fn send_flow<W>(
        &mut self,
        writer: &mut W,
        delivery_count: Option<SequenceNo>,
        available: Option<u32>,
        echo: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin;

    /// Send message via transfer frame and return whether the message is already settled
    async fn send_payload<W, Fut>(
        &mut self,
        writer: &mut W,
        detached: Fut,
        payload: Payload,
        message_format: MessageFormat,
        settled: Option<bool>,
        // The delivery state from sender is useful for
        // 1. link resumption
        // 2. transaction
        // The delivery state should be attached on every transfer if specified
        state: Option<DeliveryState>,
        batchable: bool,
    ) -> Result<Settlement, Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
        Fut: Future<Output = Option<LinkFrame>> + Send;

    async fn dispose<W>(
        &mut self,
        writer: &mut W,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin;

    async fn batch_dispose<W>(
        &mut self,
        writer: &mut W,
        ids_and_tags: Vec<(DeliveryNumber, DeliveryTag)>,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin;
}

#[async_trait]
pub(crate) trait ReceiverLink: Link + LinkExt {
    type Error: Send;

    const ROLE: Role = Role::Receiver;

    /// Set and send flow state
    async fn send_flow<W>(
        &mut self,
        writer: &mut W,
        link_credit: Option<u32>,
        drain: Option<bool>, // TODO: Is Option necessary?
        echo: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin;

    async fn on_incomplete_transfer(
        &mut self,
        delivery_tag: DeliveryTag,
        section_number: u32,
        section_offset: u64,
    );

    // More than one transfer frames should be hanlded by the
    // `Receiver`
    async fn on_incoming_transfer<T>(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<
        (
            Delivery<T>,
            Option<(DeliveryNumber, DeliveryTag, DeliveryState)>,
        ),
        Self::Error,
    >
    where
        T: for<'de> serde::Deserialize<'de> + Send;

    async fn dispose<W>(
        &mut self,
        writer: &mut W,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        // settled: bool, // TODO: This should depend on ReceiverSettleMode?
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin;
}
