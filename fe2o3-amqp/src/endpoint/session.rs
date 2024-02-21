//! Defines traits for session implementations

use std::future::Future;

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::Error,
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
};

use tokio::sync::mpsc;

use crate::{
    link::LinkRelay,
    session::frame::{SessionFrame, SessionOutgoingItem},
    Payload,
};

use super::{IncomingChannel, InputHandle, LinkFlow, OutgoingChannel, OutputHandle};

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

    fn on_incoming_attach(&mut self, attach: Attach) -> impl Future<Output = Result<(), Self::Error>> + Send + '_;

    /// An `Ok(Some(link_flow))` means an immediate echo of the link flow is requested
    fn on_incoming_flow(
        &mut self,
        flow: Flow,
    ) -> impl Future<Output = Result<Option<SessionOutgoingItem>, Self::Error>> + Send + '_;

    fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> impl Future<Output = Result<Option<Disposition>, Self::Error>> + Send + '_;

    /// An `Ok(Some(Disposition))` means an immediate disposition should be sent back
    fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<Option<Vec<Disposition>>, Self::Error>;

    fn on_incoming_detach(&mut self, detach: Detach) -> impl Future<Output = Result<(), Self::Error>> + Send + '_;

    fn on_incoming_end(&mut self, channel: IncomingChannel, end: End)
        -> Result<(), Self::EndError>;

    // Handling SessionFrames
    fn send_begin<'a>(
        &'a mut self,
        writer: &'a mpsc::Sender<SessionFrame>,
    ) -> impl Future<Output = Result<(), Self::BeginError>> + Send + 'a;

    fn send_end<'a>(
        &'a mut self,
        writer: &'a mpsc::Sender<SessionFrame>,
        error: Option<Error>,
    ) -> impl Future<Output = Result<(), Self::EndError>> + Send + 'a;

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

pub(crate) trait SessionExt: Session {
    // fn control(&self) -> &mpsc::Sender<SessionControl>;
}
