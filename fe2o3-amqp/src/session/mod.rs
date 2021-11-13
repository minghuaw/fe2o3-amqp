use async_trait::async_trait;
use bytes::BytesMut;
use fe2o3_amqp_types::{
    definitions::{Error, Fields, Handle, SequenceNo, TransferNumber},
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
    primitives::Symbol,
};
use slab::Slab;
use tokio::{sync::{mpsc::{self, Sender}, oneshot}, task::JoinHandle};

use crate::{
    connection::ConnectionHandle,
    control::SessionControl,
    endpoint,
    error::EngineError,
    link::{LinkFrame, LinkIncomingItem},
};

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

    pub(crate) async fn create_link(&mut self, tx: Sender<LinkIncomingItem>) -> Result<Handle, EngineError> {
        let (responder, resp_rx) = oneshot::channel();
        self.control
            .send(SessionControl::CreateLink {tx, responder})
            .await?;
        let result = resp_rx.await
            .map_err(|_| EngineError::Message("Oneshot sender is dropped"))?;
        result
    }

    pub(crate) async fn drop_link(&mut self, handle: Handle) -> Result<(), EngineError> {
        self.control
            .send(SessionControl::DropLink(handle))
            .await?;
        Ok(())
    }
}

pub struct Session {
    control: mpsc::Sender<SessionControl>,
    // session_id: usize,
    outgoing_channel: u16,

    // local amqp states
    local_state: SessionState,
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

    // local links
    local_links: Slab<Sender<LinkIncomingItem>>,
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

    fn local_state(&self) -> &Self::State {
        &self.local_state
    }

    fn local_state_mut(&mut self) -> &mut Self::State {
        &mut self.local_state
    }

    fn create_link(&mut self, tx: Sender<LinkIncomingItem>) -> Result<Handle, EngineError> {
        match &self.local_state {
            SessionState::Mapped => {},
            _ => return Err(EngineError::Message("Illegal session local state"))
        };

        // get a new entry index
        let entry = self.local_links.vacant_entry();
        let handle = Handle(entry.key() as u32);

        // check if handle max is exceeded
        if handle.0 > self.handle_max.0 {
            Err(EngineError::Message(
                "Handle max exceeded"
            ))
        } else {
            entry.insert(tx);
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
        channel: u16,
        attach: Attach,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_flow(&mut self, channel: u16, flow: Flow) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_transfer(
        &mut self,
        channel: u16,
        transfer: Transfer,
        payload: Option<BytesMut>,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_disposition(
        &mut self,
        channel: u16,
        disposition: Disposition,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_detach(
        &mut self,
        channel: u16,
        detach: Detach,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_end(&mut self, channel: u16, end: End) -> Result<(), Self::Error> {
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

    async fn send_begin(
        &mut self,
        writer: &mut mpsc::Sender<SessionFrame>,
    ) -> Result<(), Self::Error> {
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
                writer.send(frame).await?;
                self.local_state = SessionState::BeginSent;
            }
            SessionState::BeginReceived => {
                writer.send(frame).await?;
                self.local_state = SessionState::Mapped;
            }
            _ => return Err(EngineError::Message("Illegal local state")),
        }

        Ok(())
    }

    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error> {
        // TODO: is state checking redundant?

        println!(">>> Debug: on_outgoing_attach");
        let body = SessionFrameBody::Attach(attach);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_flow(&mut self, flow: Flow) -> Result<SessionFrame, Self::Error> {
        // TODO: what else do we need to do here?

        println!(">>> Debug: on_outgoing_flow");
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

        println!(">>> Debug: on_outgoing_transfer");
        let body = SessionFrameBody::Transfer {
            performative: transfer,
            payload
        };
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error> {
        // TODO: what else do we need to do here?

        println!(">>> Debug: on_outgoing_disposition");
        let body = SessionFrameBody::Disposition(disposition);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_detach(&mut self, detach: Detach) -> Result<SessionFrame, Self::Error> {
        // TODO: what else do we need to do here?

        println!(">>> Debug: on_outgoing_detach");
        let body = SessionFrameBody::Detach(detach);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    async fn send_end(
        &mut self,
        writer: &mut mpsc::Sender<SessionFrame>,
        error: Option<Error>,
    ) -> Result<(), Self::Error> {
        match self.local_state {
            SessionState::Mapped => match error.is_some() {
                true => self.local_state = SessionState::Discarding,
                false => self.local_state = SessionState::EndSent,
            },
            SessionState::EndReceived => self.local_state = SessionState::Unmapped,
            _ => return Err(EngineError::Message("Illegal local state")),
        }

        let frame = SessionFrame::new(self.outgoing_channel, SessionFrameBody::End(End { error }));
        writer.send(frame).await?;
        Ok(())
    }
}
