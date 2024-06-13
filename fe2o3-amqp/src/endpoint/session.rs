//! Defines traits for session implementations

use std::future::Future;

use fe2o3_amqp_types::{
    definitions::Error,
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
};

use tokio::sync::mpsc;

use crate::{
    link::LinkRelay,
    session::frame::{SessionFrame, SessionOutgoingItem},
    Payload, SendBound,
};

use super::{IncomingChannel, InputHandle, LinkFlow, OutgoingChannel, OutputHandle};

pub(crate) trait Session {
    type AllocError: SendBound;
    type BeginError: SendBound;
    type EndError: SendBound;
    type Error: SendBound;
    type State;

    fn local_state(&self) -> &Self::State;

    fn outgoing_channel(&self) -> OutgoingChannel;

    // Allocate new local handle for new Link
    fn allocate_link(
        &mut self,
        link_name: String,
        link_relay: Option<LinkRelay<()>>,
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

    fn on_incoming_attach(
        &mut self,
        attach: Attach,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// An `Ok(Some(link_flow))` means an immediate echo of the link flow is requested
    fn on_incoming_flow(
        &mut self,
        flow: Flow,
    ) -> impl Future<Output = Result<Option<SessionOutgoingItem>, Self::Error>> + Send;

    fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> impl Future<Output = Result<Option<Disposition>, Self::Error>> + Send;

    /// An `Ok(Some(Disposition))` means an immediate disposition should be sent back
    fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<Option<Vec<Disposition>>, Self::Error>;

    fn on_incoming_detach(
        &mut self,
        detach: Detach,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn on_incoming_end(&mut self, channel: IncomingChannel, end: End)
        -> Result<(), Self::EndError>;

    // Handling SessionFrames
    async fn send_begin(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
    ) -> Result<(), Self::BeginError>;

    fn send_end(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
        error: Option<Error>,
    ) -> impl Future<Output = Result<(), Self::EndError>> + Send;

    // Intercepting LinkFrames
    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error>;

    fn on_outgoing_flow(&mut self, flow: LinkFlow) -> Result<SessionFrame, Self::Error>;

    fn on_outgoing_transfer(
        &mut self,
        input_handle: InputHandle,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<SessionOutgoingItem>, Self::Error>;

    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error>;

    fn on_outgoing_detach(&mut self, detach: Detach) -> SessionFrame;
}