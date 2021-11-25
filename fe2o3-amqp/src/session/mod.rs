use std::collections::BTreeMap;

use async_trait::async_trait;
use bytes::BytesMut;
use fe2o3_amqp_types::{
    definitions::{Error, Fields, Handle, SequenceNo, TransferNumber},
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
    primitives::Symbol,
};
use futures_util::{Sink, SinkExt};
use slab::Slab;
use tokio::{
    sync::{
        mpsc::{self, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use crate::{connection::ConnectionHandle, control::SessionControl, endpoint::{self, LinkFlow}, error::EngineError, link::{LinkFrame, LinkHandle, LinkIncomingItem}, util::Constant};

mod frame;
pub use frame::*;
pub mod builder;
pub mod engine;

// 2.5.5 Session States
// UNMAPPED
// BEGIN SENT
// BEGIN RCVD
// MAPPED END SENT
// END RCVD
// DISCARDING
pub enum SessionState {
    Unmapped,

    BeginSent,

    BeginReceived,

    Mapped,

    EndSent,

    EndReceived,

    Discarding,
}

/// This is not necessary as these will just stay in the SessionEngine
/// and not shared with other tasks/threads
// pub struct SessionFlowState {
//     next_incoming_id: TransferNumber,
//     incoming_window: u32,
//     next_outgoing_id: TransferNumber,
//     outgoing_window: u32,
// }

pub struct SessionHandle {
    control: mpsc::Sender<SessionControl>,
    handle: JoinHandle<Result<(), EngineError>>,

    // outgoing for Link
    pub(crate) outgoing: mpsc::Sender<LinkFrame>,
}

impl SessionHandle {
    pub async fn end(&mut self) -> Result<(), EngineError> {
        self.control.send(SessionControl::End(None)).await?;
        match (&mut self.handle).await {
            Ok(res) => res,
            Err(_) => Err(EngineError::Message("JoinError")),
        }
    }

    pub(crate) async fn create_link(
        &mut self,
        link_handle: LinkHandle,
    ) -> Result<Handle, EngineError> {
        let (responder, resp_rx) = oneshot::channel();
        self.control
            .send(SessionControl::CreateLink { link_handle, responder })
            .await?;
        let result = resp_rx
            .await
            .map_err(|_| EngineError::Message("Oneshot sender is dropped"))?;
        result
    }

    // pub(crate) async fn drop_link(&mut self, handle: Handle) -> Result<(), EngineError> {
    //     self.control.send(SessionControl::DropLink(handle)).await?;
    //     Ok(())
    // }
}

pub struct Session {
    control: mpsc::Sender<SessionControl>,
    // session_id: usize,
    outgoing_channel: u16,

    // local amqp states
    local_state: SessionState,
    initial_outgoing_id: Constant<TransferNumber>,
    next_outgoing_id: TransferNumber,
    incoming_window: TransferNumber,
    outgoing_window: TransferNumber,
    handle_max: Handle,

    // remote amqp states
    incoming_channel: Option<u16>,
    // initialize with 0 first and change after receiving the remote Begin
    next_incoming_id: TransferNumber,
    remote_incoming_window: SequenceNo,
    remote_outgoing_window: SequenceNo,

    // capabilities
    offered_capabilities: Option<Vec<Symbol>>,
    desired_capabilities: Option<Vec<Symbol>>,
    properties: Option<Fields>,

    /// local links by output handle
    local_links: Slab<LinkHandle>,
    link_by_name: BTreeMap<String, Handle>,
    link_by_input_handle: BTreeMap<Handle, Handle>,
}

impl Session {
    /// Alias for `begin`
    pub async fn new(conn: &mut ConnectionHandle) -> Result<SessionHandle, EngineError> {
        Self::begin(conn).await
    }

    pub fn builder() -> builder::Builder {
        builder::Builder::new()
    }

    pub async fn begin(conn: &mut ConnectionHandle) -> Result<SessionHandle, EngineError> {
        Session::builder().begin(conn).await
    }
}

#[async_trait]
impl endpoint::Session for Session {
    type Error = EngineError;
    type State = SessionState;
    type LinkHandle = LinkHandle;

    fn local_state(&self) -> &Self::State {
        &self.local_state
    }

    fn local_state_mut(&mut self) -> &mut Self::State {
        &mut self.local_state
    }

    fn create_link(&mut self, link_handle: LinkHandle) -> Result<Handle, EngineError> {
        match &self.local_state {
            SessionState::Mapped => {}
            _ => return Err(EngineError::Message("Illegal session local state")),
        };

        // get a new entry index
        let entry = self.local_links.vacant_entry();
        let handle = Handle(entry.key() as u32);

        // check if handle max is exceeded
        if handle.0 > self.handle_max.0 {
            Err(EngineError::Message("Handle max exceeded"))
        } else {
            entry.insert(link_handle);
            // TODO: how to know which link to send the Flow frames to?
            Ok(handle)
        }
    }

    fn drop_link(&mut self, handle: Handle) {
        todo!()
    }

    async fn on_incoming_begin(&mut self, channel: u16, begin: Begin) -> Result<(), Self::Error> {
        match self.local_state {
            SessionState::Unmapped => self.local_state = SessionState::BeginReceived,
            SessionState::BeginSent => self.local_state = SessionState::Mapped,
            _ => return Err(EngineError::illegal_state()),
        }

        self.incoming_channel = Some(channel);
        self.next_incoming_id = begin.next_outgoing_id;
        self.remote_incoming_window = begin.incoming_window;
        self.remote_outgoing_window = begin.outgoing_window;

        Ok(())
    }

    async fn on_incoming_attach(
        &mut self,
        _channel: u16,
        attach: Attach,
    ) -> Result<(), Self::Error> {
        println!(">>> Debug: Session::on_incoming_attach");
        // look up link Handle by link name
        match self.link_by_name.get(&attach.name) {
            Some(output_handle) => match self.local_links.get_mut(output_handle.0 as usize) {
                Some(link) => {
                    println!(">>> Debug: found local link");
                    let input_handle = attach.handle.clone(); // handle is just a wrapper around u32
                    self.link_by_input_handle.insert(input_handle, output_handle.clone());
                    link.tx.send(LinkFrame::Attach(attach)).await?;
                }
                None => {
                    todo!()
                }
            },
            None => {
                todo!()
            }
        }

        Ok(())
    }

    async fn on_incoming_flow(&mut self, channel: u16, flow: Flow) -> Result<(), Self::Error> {
        println!(">>> Debug: Session::on_incoming_flow");

        // TODO: handle session flow control
        // When the endpoint receives a flow frame from its peer, it MUST update the next-incoming-id 
        // directly from the next-outgoing-id of the frame, and it MUST update the remote-outgoing- 
        // window directly from the outgoing-window of the frame.
        self.next_incoming_id = flow.next_outgoing_id;
        self.remote_outgoing_window = flow.outgoing_window;

        match &flow.next_incoming_id {
            Some(flow_next_incoming_id) => {
                // The remote-incoming-window is computed as follows: 
                // next-incoming-id_flow + incoming-window_flow - next-outgoing-id_endpoint
                self.remote_incoming_window = flow_next_incoming_id + flow.incoming_window - self.next_outgoing_id;
            },
            None => {
                // If the next-incoming-id field of the flow frame is not set, then remote-incoming-window is computed as follows:
                // initial-outgoing-id_endpoint + incoming-window_flow - next-outgoing-id_endpoint
                self.remote_incoming_window = *(self.initial_outgoing_id.value()) + flow.incoming_window - self.next_outgoing_id;
            }
        }

        // TODO: handle link flow control
        if let Some(input_handle) = &flow.handle { 
            match self.link_by_input_handle.get(input_handle) {
                Some(output_handle) => {
                    match self.local_links.get_mut(output_handle.0 as usize) {
                        Some(link_handle) => {
                            let link_flow = LinkFlow::try_from_flow(&flow)
                                .ok_or_else(|| EngineError::Message("Expecting link flow found empty field"))?;
                            let echo = link_handle.state.on_incoming_flow(link_flow)?;
                            if let Some(echo_flow) = echo {
                                todo!()
                            }
                        },
                        None => return Err(EngineError::unattached_handle())
                    }
                },
                None => return Err(EngineError::unattached_handle())
            }
        }

        Ok(())
    }

    async fn on_incoming_transfer(
        &mut self,
        channel: u16,
        transfer: Transfer,
        payload: Option<BytesMut>,
    ) -> Result<(), Self::Error> {
        println!(">>> Debug: Session::on_incoming_transfer");
        todo!()
    }

    async fn on_incoming_disposition(
        &mut self,
        channel: u16,
        disposition: Disposition,
    ) -> Result<(), Self::Error> {
        println!(">>> Debug: Session::on_incoming_disposition");
        todo!()
    }

    async fn on_incoming_detach(
        &mut self,
        channel: u16,
        detach: Detach,
    ) -> Result<(), Self::Error> {
        println!(">>> Debug: Session::on_incoming_detach");
        todo!()
    }

    async fn on_incoming_end(&mut self, channel: u16, end: End) -> Result<(), Self::Error> {
        println!(">>> Debug: Session::on_incoming_end");
        if Some(channel) != self.incoming_channel {
            return Err(EngineError::Message("Incoming channel mismatch"));
        }

        match self.local_state {
            SessionState::Mapped => {
                self.local_state = SessionState::EndReceived;
                self.control.send(SessionControl::End(None)).await?;
            }
            SessionState::Discarding | SessionState::EndSent => {
                self.local_state = SessionState::Unmapped
            }
            _ => return Err(EngineError::illegal_state()),
        }

        if let Some(error) = end.error {
            println!(">>> Debug: Remote End carries error: {:?}", error);
        }

        Ok(())
    }

    async fn send_begin<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame> + Send + Unpin,
        W::Error: Into<EngineError>,
    {
        println!(">>> Debug: Session::send_begin");
        let begin = Begin {
            remote_channel: self.incoming_channel,
            next_outgoing_id: self.next_outgoing_id,
            incoming_window: self.incoming_window,
            outgoing_window: self.outgoing_window,
            handle_max: self.handle_max.clone(),
            offered_capabilities: self.offered_capabilities.clone(),
            desired_capabilities: self.desired_capabilities.clone(),
            properties: self.properties.clone(),
        };
        let frame = SessionFrame::new(self.outgoing_channel, SessionFrameBody::Begin(begin));

        // check local states
        match &self.local_state {
            SessionState::Unmapped => {
                writer.send(frame).await.map_err(Into::into)?;
                self.local_state = SessionState::BeginSent;
            }
            SessionState::BeginReceived => {
                writer.send(frame).await.map_err(Into::into)?;
                self.local_state = SessionState::Mapped;
            }
            _ => return Err(EngineError::Message("Illegal local state")),
        }

        Ok(())
    }

    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error> {
        // TODO: is state checking redundant?

        println!(">>> Debug: Session::on_outgoing_attach");
        let name = attach.name.clone();
        let handle = attach.handle.clone();
        match self.link_by_name.contains_key(&name) {
            true => return Err(EngineError::Message("Link name must be unique")),
            false => self.link_by_name.insert(name, handle),
        };

        let body = SessionFrameBody::Attach(attach);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_flow(&mut self, flow: Flow) -> Result<SessionFrame, Self::Error> {
        // TODO: what else do we need to do here?

        println!(">>> Debug: Session::on_outgoing_flow");
        let body = SessionFrameBody::Flow(flow);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_transfer(
        &mut self,
        transfer: Transfer,
        payload: Option<BytesMut>,
    ) -> Result<SessionFrame, Self::Error> {
        // TODO: what else do we need to do here

        println!(">>> Debug: Session::on_outgoing_transfer");
        let body = SessionFrameBody::Transfer {
            performative: transfer,
            payload,
        };
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error> {
        // TODO: what else do we need to do here?

        println!(">>> Debug: Session::on_outgoing_disposition");
        let body = SessionFrameBody::Disposition(disposition);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_detach(&mut self, detach: Detach) -> Result<SessionFrame, Self::Error> {
        // TODO: what else do we need to do here?

        println!(">>> Debug: Session::on_outgoing_detach");
        let body = SessionFrameBody::Detach(detach);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    async fn send_end<W>(&mut self, writer: &mut W, error: Option<Error>) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame> + Send + Unpin,
        W::Error: Into<EngineError>,
    {
        match self.local_state {
            SessionState::Mapped => match error.is_some() {
                true => self.local_state = SessionState::Discarding,
                false => self.local_state = SessionState::EndSent,
            },
            SessionState::EndReceived => self.local_state = SessionState::Unmapped,
            _ => return Err(EngineError::Message("Illegal local state")),
        }

        let frame = SessionFrame::new(self.outgoing_channel, SessionFrameBody::End(End { error }));
        writer.send(frame).await.map_err(Into::into)?;
        Ok(())
    }
}
