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
use bytes::BytesMut;
use fe2o3_amqp_types::{
    definitions::{self, Error, Handle, Role, SequenceNo},
    performatives::{Attach, Begin, Close, Detach, Disposition, End, Flow, Open, Transfer},
    primitives::{Boolean, UInt}, messaging::Message,
};
use futures_util::Sink;
use tokio::sync::mpsc;

use crate::{
    connection::engine::SessionId,
    link::LinkFrame,
    session::{SessionFrame, SessionIncomingItem},
    transport::amqp::Frame,
};

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
    ) -> Result<(u16, SessionId), Self::AllocError>;
    // Remove outgoing id and session id association
    fn deallocate_session(&mut self, session_id: usize);

    // async fn forward_to_session(&mut self, incoming_channel: u16, frame: SessionFrame) -> Result<(), Self::Error>;

    /// Reacting to remote Open frame
    async fn on_incoming_open(&mut self, channel: u16, open: Open) -> Result<(), Self::Error>;

    /// Reacting to remote Begin frame
    ///
    /// Do NOT forward to session here. Forwarding is handled elsewhere.
    async fn on_incoming_begin(&mut self, channel: u16, begin: Begin) -> Result<(), Self::Error>;

    /// Reacting to remote End frame
    async fn on_incoming_end(&mut self, channel: u16, end: End) -> Result<(), Self::Error>;

    /// Reacting to remote Close frame
    async fn on_incoming_close(
        &mut self,
        channel: u16,
        close: Close,
    ) -> Result<Option<definitions::Error>, Self::Error>;

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
    fn on_outgoing_begin(&mut self, channel: u16, begin: Begin) -> Result<Frame, Self::Error>;

    fn on_outgoing_end(&mut self, channel: u16, end: End) -> Result<Frame, Self::Error>;

    fn session_tx_by_incoming_channel(
        &mut self,
        channel: u16,
    ) -> Option<&mut mpsc::Sender<SessionIncomingItem>>;

    fn session_tx_by_outgoing_channel(
        &mut self,
        channel: u16,
    ) -> Option<&mut mpsc::Sender<SessionIncomingItem>>;
}

#[async_trait]
pub trait Session {
    type AllocError: Send;
    type Error: Send;
    type State;
    type LinkHandle;

    fn local_state(&self) -> &Self::State;
    fn local_state_mut(&mut self) -> &mut Self::State;

    // Allocate new local handle for new Link
    fn allocate_link(
        &mut self,
        link_name: String,
        link_handle: Self::LinkHandle,
    ) -> Result<Handle, Self::AllocError>;
    fn deallocate_link(&mut self, link_name: String);

    async fn on_incoming_begin(&mut self, channel: u16, begin: Begin) -> Result<(), Self::Error>;
    async fn on_incoming_attach(&mut self, channel: u16, attach: Attach)
        -> Result<(), Self::Error>;
    async fn on_incoming_flow(&mut self, channel: u16, flow: Flow) -> Result<(), Self::Error>;
    async fn on_incoming_transfer(
        &mut self,
        channel: u16,
        transfer: Transfer,
        payload: Option<BytesMut>,
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
        W: Sink<SessionFrame, Error = mpsc::error::SendError<SessionFrame>> + Send + Unpin;

    async fn send_end<W>(
        &mut self,
        writer: &mut W,
        error: Option<Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame, Error = mpsc::error::SendError<SessionFrame>> + Send + Unpin;

    // Intercepting LinkFrames
    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error>;
    fn on_outgoing_flow(&mut self, flow: Flow) -> Result<SessionFrame, Self::Error>;
    fn on_outgoing_transfer(
        &mut self,
        transfer: Transfer,
        payload: Option<BytesMut>,
    ) -> Result<SessionFrame, Self::Error>;
    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error>;
    fn on_outgoing_detach(&mut self, detach: Detach) -> Result<SessionFrame, Self::Error>;
}

#[async_trait]
pub trait Link {
    type DetachError: Send;
    type Error: Send;

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error>;

    /// Reacting to incoming flow should be entirely handled by session
    // async fn on_incoming_flow(&mut self, flow: Flow) -> Result<(), Self::Error>;

    // Only the receiver is supposed to receive incoming Transfer frame

    async fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<(), Self::Error>;
    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::DetachError>;

    async fn send_attach<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame, Error = mpsc::error::SendError<LinkFrame>> + Send + Unpin;

    async fn send_flow<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame, Error = mpsc::error::SendError<LinkFrame>> + Send + Unpin;

    async fn send_disposition<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame, Error = mpsc::error::SendError<LinkFrame>> + Send + Unpin;

    async fn send_detach<W>(&mut self, writer: &mut W, closed: bool, error: Option<Error>) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame, Error = mpsc::error::SendError<LinkFrame>> + Send + Unpin;

    // async fn on_incoming_transfer(&mut self, transfer: Transfer, payload: Option<BytesMut>) -> Result<(), Self::Error>;
    // async fn on_outgoing_flow() -> Result<(), Self::Error>;
    // async fn on_outgoing_transfer() -> Result<(), Self::Error>;
    // async fn on_outgoing_disposition() -> Result<(), Self::Error>;
}

// #[async_trait]
// pub trait LinkFlowControl {
//     type Error;

//     /// TODO: redundant?
//     fn on_outgoing_flow(&mut self, flow: &Flow) -> Result<(), Self::Error>;

//     fn on_incoming_flow(&mut self, flow: &Flow) -> Result<Option<Flow>, Self::Error>;
// }

/// A subset of the fields in the Flow performative
#[derive(Debug, Default)]
pub struct LinkFlow {
    pub delivery_count: Option<SequenceNo>,
    pub link_credit: Option<UInt>,
    pub available: Option<UInt>,
    pub drain: Boolean,
    pub echo: Boolean,
}

impl From<&Flow> for LinkFlow {
    fn from(flow: &Flow) -> Self {
        LinkFlow {
            delivery_count: flow.delivery_count,
            link_credit: flow.link_credit,
            available: flow.available,
            drain: flow.drain,
            echo: flow.echo,
        }
    }
}

#[async_trait]
pub trait SenderLink: Link {
    const ROLE: Role = Role::Sender;

    async fn send_transfer<W, M>(&mut self, writer: &mut W, message: M) -> Result<(), <Self as Link>::Error>
    where
        W: Sink<LinkFrame, Error = mpsc::error::SendError<LinkFrame>> + Send + Unpin,
        M: Into<Message> + Send;
}

#[async_trait]
pub trait ReceiverLink: Link {
    const ROLE: Role = Role::Receiver;

    async fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Option<BytesMut>,
    ) -> Result<(), <Self as Link>::Error>;
}
