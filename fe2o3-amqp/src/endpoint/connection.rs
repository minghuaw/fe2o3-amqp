//! Defines trait for connection implementations

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::Error,
    performatives::{Begin, Close, End, Open},
};
use futures_util::Sink;
use tokio::sync::mpsc;

use crate::{frames::amqp::Frame, session::frame::SessionIncomingItem, SendBound};

use super::{IncomingChannel, OutgoingChannel, Session};

/// Trait for connection
#[cfg_attr(not(target_arch="wasm32"), async_trait)]
#[cfg_attr(target_arch="wasm32", async_trait(?Send))]
pub(crate) trait Connection {
    type AllocError: SendBound;
    type OpenError: SendBound;
    type CloseError: SendBound;
    type Error: SendBound;
    type State: SendBound;
    type Session: Session + SendBound;

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
    fn on_incoming_open(
        &mut self,
        channel: IncomingChannel,
        open: Open,
    ) -> Result<(), Self::OpenError>;

    /// Reacting to remote Begin frame
    ///
    /// Do NOT forward to session here. Forwarding is handled elsewhere.
    async fn on_incoming_begin(
        &mut self,
        channel: IncomingChannel,
        begin: Begin,
    ) -> Result<(), Self::Error>;

    /// Reacting to remote End frame
    async fn on_incoming_end(
        &mut self,
        channel: IncomingChannel,
        end: End,
    ) -> Result<(), Self::Error>;

    /// Reacting to remote Close frame
    fn on_incoming_close(
        &mut self,
        channel: IncomingChannel,
        close: Close,
    ) -> Result<(), Self::CloseError>;

    /// Sending out an Open frame
    ///
    /// The write is passed in is because sending an Open frame also changes the local
    /// connection state. If the sending fails outside, coming back
    /// and revert the state changes would be too complicated
    async fn send_open<W>(&mut self, writer: &mut W) -> Result<(), Self::OpenError>
    where
        W: Sink<Frame> + SendBound + Unpin,
        Self::OpenError: From<W::Error>; // DO NOT remove this. This is where `Transport` will be used

    async fn send_close<W>(
        &mut self,
        writer: &mut W,
        error: Option<Error>,
    ) -> Result<(), Self::CloseError>
    where
        W: Sink<Frame> + SendBound + Unpin,
        Self::CloseError: From<W::Error>; // DO NOT remove this. This is where `Transport` will be used

    /// Intercepting session frames
    fn on_outgoing_begin(
        &mut self,
        channel: OutgoingChannel,
        begin: Begin,
    ) -> Result<Frame, Self::Error>;

    fn on_outgoing_end(&mut self, channel: OutgoingChannel, end: End)
        -> Result<Frame, Self::Error>;

    fn session_tx_by_incoming_channel(
        &mut self,
        incoming_channel: IncomingChannel,
    ) -> Option<&mpsc::Sender<SessionIncomingItem>>;

    fn session_tx_by_outgoing_channel(
        &mut self,
        outgoing_channel: OutgoingChannel,
    ) -> Option<&mpsc::Sender<SessionIncomingItem>>;
}
