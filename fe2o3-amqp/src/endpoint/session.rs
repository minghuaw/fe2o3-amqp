//! Defines traits for session implementations

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::Error,
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
};

use tokio::sync::mpsc;

use crate::{control::SessionControl, link::LinkRelay, session::frame::SessionFrame, Payload};

use super::{IncomingChannel, InputHandle, LinkFlow, OutgoingChannel, OutputHandle};

#[async_trait]
pub(crate) trait Session {
    type AllocError: Send;
    type BeginError: Send;
    type EndError: Send;
    type Error: Send;
    type State;

    fn local_state(&self) -> &Self::State;

    fn local_state_mut(&mut self) -> &mut Self::State;

    fn outgoing_channel(&self) -> OutgoingChannel;

    // Allocate new local handle for new Link
    fn allocate_link(
        &mut self,
        link_name: String,
        link_relay: Option<LinkRelay<()>>, // TODO: how to expose error at compile time?
    ) -> Result<OutputHandle, Self::AllocError>;

    fn allocate_incoming_link(
        &mut self,
        link_name: String,
        link_relay: LinkRelay<()>,
        input_handle: InputHandle,
    ) -> Result<OutputHandle, Self::AllocError>;

    fn deallocate_link(&mut self, output_handle: OutputHandle);

    fn on_incoming_begin(
        &mut self,
        channel: IncomingChannel,
        begin: Begin,
    ) -> Result<(), Self::BeginError>;

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error>;

    async fn on_incoming_flow(&mut self, flow: Flow) -> Result<(), Self::Error>;

    async fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<(), Self::Error>;

    async fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<(), Self::Error>;

    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::Error>;

    async fn on_incoming_end(
        &mut self,
        channel: IncomingChannel,
        end: End,
    ) -> Result<(), Self::EndError>;

    // Handling SessionFrames
    async fn send_begin(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
    ) -> Result<(), Self::BeginError>;

    async fn send_end(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
        error: Option<Error>,
    ) -> Result<(), Self::EndError>;

    // Intercepting LinkFrames
    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error>;

    fn on_outgoing_flow(&mut self, flow: LinkFlow) -> Result<SessionFrame, Self::Error>;

    fn on_outgoing_transfer(
        &mut self,
        input_handle: InputHandle,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<SessionFrame, Self::Error>;

    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error>;

    fn on_outgoing_detach(&mut self, detach: Detach) -> SessionFrame;
}

pub(crate) trait SessionExt: Session {
    fn control(&self) -> &mpsc::Sender<SessionControl>;
}
