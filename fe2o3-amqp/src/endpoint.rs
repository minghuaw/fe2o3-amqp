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
    definitions::{Error, Handle, Role},
    performatives::{Attach, Begin, Close, Detach, Disposition, End, Flow, Open, Transfer},
};
use futures_util::Sink;
use tokio::sync::mpsc;

use crate::{
    connection::engine::SessionId,
    error::EngineError,
    link::{LinkFrame, LinkIncomingItem},
    session::{SessionFrame, SessionIncomingItem},
    transport::amqp::Frame,
};

#[async_trait]
pub trait Connection {
    type Error: Into<EngineError> + Send;
    type State: Send;
    type Session: Session + Send;

    fn local_state(&self) -> &Self::State;
    fn local_state_mut(&mut self) -> &mut Self::State;
    fn local_open(&self) -> &Open;

    // Allocate outgoing channel id and session id to a new session
    fn create_session(
        &mut self,
        tx: mpsc::Sender<SessionIncomingItem>,
    ) -> Result<(u16, SessionId), Self::Error>;
    // Remove outgoing id and session id association
    fn drop_session(&mut self, session_id: usize);

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
    async fn on_incoming_close(&mut self, channel: u16, close: Close) -> Result<(), Self::Error>;

    /// Sending out an Open frame
    ///
    /// The write is passed in is because sending an Open frame also changes the local
    /// connection state. If the sending fails outside, coming back
    /// and revert the state changes would be too complicated
    async fn send_open<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<Frame, Error = EngineError> + Send + Unpin;

    async fn send_close<W>(
        &mut self,
        writer: &mut W,
        error: Option<Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<Frame, Error = EngineError> + Send + Unpin;

    /// Intercepting session frames
    fn on_outgoing_begin(&mut self, channel: u16, begin: Begin)
        -> Result<Frame, Self::Error>;

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
    type Error: Into<EngineError>;
    type State;

    fn local_state(&self) -> &Self::State;
    fn local_state_mut(&mut self) -> &mut Self::State;

    // Allocate new local handle for new Link
    fn create_link(&mut self, tx: mpsc::Sender<LinkIncomingItem>) -> Result<Handle, EngineError>;
    fn drop_link(&mut self, handle: Handle);

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
    async fn send_begin(
        &mut self,
        writer: &mut mpsc::Sender<SessionFrame>,
    ) -> Result<(), Self::Error>;
    async fn send_end(
        &mut self,
        writer: &mut mpsc::Sender<SessionFrame>,
        error: Option<Error>,
    ) -> Result<(), Self::Error>;

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
    type Error: Into<EngineError>;

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error>;

    async fn on_incoming_flow(&mut self, flow: Flow) -> Result<(), Self::Error>;

    // Only the receiver is supposed to receive incoming Transfer frame

    async fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<(), Self::Error>;
    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::Error>;

    async fn send_attach(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::Error>;

    async fn send_flow(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>
    ) -> Result<(), Self::Error>;

    async fn send_disposition(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>
    ) -> Result<(), Self::Error>;

    async fn send_detach(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::Error>;

    // async fn on_incoming_transfer(&mut self, transfer: Transfer, payload: Option<BytesMut>) -> Result<(), Self::Error>;
    // async fn on_outgoing_flow() -> Result<(), Self::Error>;
    // async fn on_outgoing_transfer() -> Result<(), Self::Error>;
    // async fn on_outgoing_disposition() -> Result<(), Self::Error>;
}

#[async_trait]
pub trait SenderLink: Link {
    const ROLE: Role = Role::Sender;

    async fn send_transfer(&mut self, writer: &mut mpsc::Sender<LinkFrame>) -> Result<(), <Self as Link>::Error>;
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
