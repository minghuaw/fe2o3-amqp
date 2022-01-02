use std::{collections::BTreeMap, io};

use async_trait::async_trait;
use bytes::Bytes;
use fe2o3_amqp_types::{
    definitions::{
        self, AmqpError, Fields, Handle, Role, SequenceNo, SessionError, TransferNumber,
    },
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
    primitives::Symbol,
};
use futures_util::{Sink, SinkExt};
use slab::Slab;
use tokio::{
    sync::{
        mpsc::{self},
        oneshot,
    },
    task::JoinHandle,
};

use crate::{
    connection::ConnectionHandle,
    control::SessionControl,
    endpoint::{self, LinkFlow},
    link::{LinkFrame, LinkHandle},
    util::Constant,
};

mod frame;
pub use frame::*;
pub mod builder;
pub mod engine;
mod error;
pub use self::error::{AllocLinkError, Error};

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
    pub(crate) control: mpsc::Sender<SessionControl>,
    engine_handle: JoinHandle<Result<(), Error>>,

    // outgoing for Link
    pub(crate) outgoing: mpsc::Sender<LinkFrame>,
}

impl SessionHandle {
    pub async fn end(&mut self) -> Result<(), Error> {
        // If sending is unsuccessful, the `SessionEngine` event loop is
        // already dropped, this should be reflected by `JoinError` then.
        let _ = self.control.send(SessionControl::End(None)).await;
        match (&mut self.engine_handle).await {
            Ok(res) => res,
            Err(e) => Err(Error::JoinError(e)),
        }
    }
}

pub(crate) async fn allocate_link(
    control: &mut mpsc::Sender<SessionControl>,
    link_name: String,
    link_handle: LinkHandle,
) -> Result<Handle, AllocLinkError> {
    let (responder, resp_rx) = oneshot::channel();

    control
        .send(SessionControl::AllocateLink {
            link_name,
            link_handle,
            responder,
        })
        .await
        // The `SendError` could only happen when the receiving half is
        // dropped, meaning the `SessionEngine::event_loop` has stopped.
        // This would also mean the `Session` is Unmapped, and thus it
        // may be treated as illegal state
        .map_err(|_| AllocLinkError::IllegalState)?;
    let result = resp_rx
        .await
        // The error could only occur when the sending half is dropped,
        // indicating the `SessionEngine::even_loop` has stopped or
        // unmapped. Thus it could be considered as illegal state
        .map_err(|_| AllocLinkError::IllegalState)?;
    result
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
    // Maps from DeliveryId to link.DeliveryCount
    delivery_tag_by_id: BTreeMap<TransferNumber, (Handle, [u8; 4])>,
}

impl Session {
    /// Alias for `begin`
    pub async fn new(conn: &mut ConnectionHandle) -> Result<SessionHandle, Error> {
        Self::begin(conn).await
    }

    pub fn builder() -> builder::Builder {
        builder::Builder::new()
    }

    pub async fn begin(conn: &mut ConnectionHandle) -> Result<SessionHandle, Error> {
        Session::builder().begin(conn).await
    }
}

#[async_trait]
impl endpoint::Session for Session {
    type AllocError = AllocLinkError;
    type Error = Error;
    type State = SessionState;
    type LinkHandle = LinkHandle;

    fn local_state(&self) -> &Self::State {
        &self.local_state
    }

    fn local_state_mut(&mut self) -> &mut Self::State {
        &mut self.local_state
    }

    fn allocate_link(
        &mut self,
        link_name: String,
        link_handle: LinkHandle,
    ) -> Result<Handle, Self::AllocError> {
        match &self.local_state {
            SessionState::Mapped => {}
            _ => return Err(AllocLinkError::IllegalState),
        };

        // check whether link name is duplciated
        if self.link_by_name.contains_key(&link_name) {
            return Err(AllocLinkError::DuplicatedLinkName);
        }

        // get a new entry index
        let entry = self.local_links.vacant_entry();
        let handle = Handle(entry.key() as u32);

        // check if handle max is exceeded
        if handle.0 > self.handle_max.0 {
            Err(AllocLinkError::HandleMaxReached)
        } else {
            entry.insert(link_handle);
            self.link_by_name.insert(link_name, handle.clone());
            // TODO: how to know which link to send the Flow frames to?
            Ok(handle)
        }
    }

    fn deallocate_link(&mut self, link_name: String) {
        if let Some(handle) = self.link_by_name.remove(&link_name) {
            self.local_links.remove(handle.0 as usize);
        }
    }

    async fn on_incoming_begin(&mut self, channel: u16, begin: Begin) -> Result<(), Self::Error> {
        match self.local_state {
            SessionState::Unmapped => self.local_state = SessionState::BeginReceived,
            SessionState::BeginSent => self.local_state = SessionState::Mapped,
            _ => return Err(AmqpError::IllegalState.into()),
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
                    // Update the receiver settle mode
                    link.receiver_settle_mode = attach.rcv_settle_mode.clone();

                    let input_handle = attach.handle.clone(); // handle is just a wrapper around u32
                    self.link_by_input_handle
                        .insert(input_handle, output_handle.clone());
                    match link.send(LinkFrame::Attach(attach)).await {
                        Ok(_) => {}
                        Err(e) => {
                            // TODO: how should this error be handled?
                            todo!()
                        }
                    }
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
                self.remote_incoming_window =
                    flow_next_incoming_id + flow.incoming_window - self.next_outgoing_id;
            }
            None => {
                // If the next-incoming-id field of the flow frame is not set, then remote-incoming-window is computed as follows:
                // initial-outgoing-id_endpoint + incoming-window_flow - next-outgoing-id_endpoint
                self.remote_incoming_window = *(self.initial_outgoing_id.value())
                    + flow.incoming_window
                    - self.next_outgoing_id;
            }
        }

        // TODO: handle link flow control
        if let Ok(link_flow) = LinkFlow::try_from(flow) {
            match self.link_by_input_handle.get(&link_flow.handle) {
                Some(output_handle) => match self.local_links.get_mut(output_handle.0 as usize) {
                    Some(link_handle) => {
                        if let Some(echo_flow) = link_handle
                            .on_incoming_flow(link_flow, output_handle.clone())
                            .await
                        {
                            self.control
                                .send(SessionControl::LinkFlow(echo_flow))
                                .await
                                // Sending control to self. This will only give an error if the receiving
                                // half is dropped. An error would thus indicate that the event loop
                                // has stopped, so it could be considered illegal state.
                                .map_err(|_| AmqpError::IllegalState)?;
                        }
                    }
                    None => return Err(SessionError::UnattachedHandle.into()),
                },
                None => return Err(SessionError::UnattachedHandle.into()),
            }
        }

        Ok(())
    }

    async fn on_incoming_transfer(
        &mut self,
        channel: u16,
        transfer: Transfer,
        payload: Bytes,
    ) -> Result<(), Self::Error> {
        // Upon receiving a transfer, the receiving endpoint will increment the next-incoming-id to
        // match the implicit transfer-id of the incoming transfer plus one, as well as decrementing the
        // remote-outgoing-window, and MAY (depending on policy) decrement its incoming-window.

        println!(">>> Debug: Session::on_incoming_transfer");

        todo!()
    }

    async fn on_incoming_disposition(
        &mut self,
        channel: u16,
        disposition: Disposition,
    ) -> Result<(), Self::Error> {
        println!(">>> Debug: Session::on_incoming_disposition");

        // TODO: what to do when session lost delivery_tag_by_id
        // and disposition only has delivery id?

        let first = disposition.first;
        let last = disposition.last.unwrap_or_else(|| first);

        let mut first_echo = first;
        let mut last_echo = first;
        let mut prev = false;

        let is_settled = match &disposition.state {
            Some(state) => disposition.settled || state.is_terminal(),
            None => disposition.settled,
        };

        if is_settled {
            for delivery_id in first..=last {
                if let Some((handle, delivery_tag)) = self.delivery_tag_by_id.remove(&delivery_id) {
                    if let Some(link_handle) = self.local_links.get_mut(handle.0 as usize) {
                        let echo = link_handle
                            .on_incoming_disposition(
                                disposition.role.clone(),
                                is_settled,
                                &disposition.state,
                                delivery_tag,
                            )
                            .await;

                        if echo == true {
                            if prev == false {
                                first_echo = delivery_id;
                            }
                            last_echo = delivery_id
                        } else if echo == false && prev == true {
                            let role = match disposition.role {
                                Role::Sender => Role::Receiver,
                                Role::Receiver => Role::Sender,
                            };
                            let last = if last_echo != first_echo {
                                Some(last_echo)
                            } else {
                                None
                            };

                            let disposition = Disposition {
                                role,
                                first: first_echo,
                                last,
                                settled: true,
                                state: None,
                                batchable: false, // No reply is really expected as this is a reply
                            };
                            self.control
                                .send(SessionControl::Disposition(disposition))
                                .await
                                .map_err(|_| AmqpError::IllegalState)?
                        }

                        prev = echo;
                    }
                }
            }
        } else {
            for delivery_id in first..last {
                if let Some((handle, delivery_tag)) = self.delivery_tag_by_id.get(&delivery_id) {
                    if let Some(link_handle) = self.local_links.get_mut(handle.0 as usize) {
                        // This is not expected to have an echo according to the spec sheet
                        let _ = link_handle
                            .on_incoming_disposition(
                                disposition.role.clone(),
                                is_settled,
                                &disposition.state,
                                *delivery_tag,
                            )
                            .await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn on_incoming_detach(
        &mut self,
        _channel: u16,
        detach: Detach,
    ) -> Result<(), Self::Error> {
        println!(">>> Debug: Session::on_incoming_detach");
        // Remove the link by input handle
        let output_handle = match self.link_by_input_handle.remove(&detach.handle) {
            Some(handle) => handle,
            None => todo!(), // End session with unattached handle?
        };
        match self.local_links.get_mut(output_handle.0 as usize) {
            Some(link) => {
                match link.send(LinkFrame::Detach(detach)).await {
                    Ok(_) => {}
                    Err(e) => todo!(), // End session with unattached handle?
                }
            }
            None => todo!(), // End session with unattached handle?
        }

        Ok(())
    }

    async fn on_incoming_end(&mut self, channel: u16, end: End) -> Result<(), Self::Error> {
        println!(">>> Debug: Session::on_incoming_end");

        match self.local_state {
            SessionState::Mapped => {
                self.local_state = SessionState::EndReceived;
                self.control
                    .send(SessionControl::End(None))
                    .await
                    // The `SendError` occurs when the receiving half is dropped,
                    // indicating that the `SessionEngine::event_loop` has stopped.
                    // and thus should yield an illegal state error
                    .map_err(|e| AmqpError::IllegalState)?;
            }
            SessionState::Discarding | SessionState::EndSent => {
                self.local_state = SessionState::Unmapped
            }
            _ => return Err(AmqpError::IllegalState.into()),
        }

        if let Some(error) = end.error {
            // TODO: handle remote error
            println!(">>> Debug: Remote End carries error: {:?}", error);
        }

        Ok(())
    }

    async fn send_begin<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame, Error = mpsc::error::SendError<SessionFrame>> + Send + Unpin,
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
                writer
                    .send(frame)
                    .await
                    // The receiving half must have dropped, and thus the `Connection`
                    // event loop has stopped. It should be treated as an io error
                    .map_err(|e| {
                        Self::Error::Io(io::Error::new(io::ErrorKind::Other, e.to_string()))
                    })?;
                self.local_state = SessionState::BeginSent;
            }
            SessionState::BeginReceived => {
                writer.send(frame).await.map_err(|e| {
                    Self::Error::Io(io::Error::new(io::ErrorKind::Other, e.to_string()))
                })?;
                self.local_state = SessionState::Mapped;
            }
            _ => return Err(AmqpError::IllegalState.into()),
        }

        Ok(())
    }

    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error> {
        // TODO: is state checking redundant?

        println!(">>> Debug: Session::on_outgoing_attach");

        let body = SessionFrameBody::Attach(attach);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_flow(&mut self, flow: LinkFlow) -> Result<SessionFrame, Self::Error> {
        // TODO: what else do we need to do here?

        println!(">>> Debug: Session::on_outgoing_flow");

        let flow = Flow {
            // Session flow states
            next_incoming_id: Some(self.next_incoming_id),
            incoming_window: self.incoming_window,
            next_outgoing_id: self.next_outgoing_id,
            outgoing_window: self.outgoing_window,
            // Link flow states
            handle: Some(flow.handle),
            delivery_count: flow.delivery_count,
            link_credit: flow.link_credit,
            available: flow.available,
            drain: flow.drain,
            echo: flow.echo,
            properties: flow.properties,
        };

        let body = SessionFrameBody::Flow(flow);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_transfer(
        &mut self,
        mut transfer: Transfer,
        payload: Bytes,
    ) -> Result<SessionFrame, Self::Error> {
        // Upon sending a transfer, the sending endpoint will increment its next-outgoing-id, decre-
        // ment its remote-incoming-window, and MAY (depending on policy) decrement its outgoing-
        // window.

        // TODO: What policy would result in a decrement in outgoing-window?

        // Only the first transfer is required to have delivery_tag and delivery_id
        if let Some(tag) = &transfer.delivery_tag {
            // The next-outgoing-id is the transfer-id to assign to the next transfer frame.
            let delivery_id = self.next_outgoing_id;
            transfer.delivery_id = Some(delivery_id);

            let mut delivery_tag = [0u8; 4];
            // TODO: Is bound check necessary
            (&mut delivery_tag).copy_from_slice(tag);

            // Disposition doesn't carry delivery tag
            self.delivery_tag_by_id
                .insert(delivery_id, (transfer.handle.clone(), delivery_tag));
        }

        self.next_outgoing_id += 1;

        // The remote-incoming-window reflects the maximum number of outgoing transfers that can
        // be sent without exceeding the remote endpoint’s incoming-window. This value MUST be
        // decremented after every transfer frame is sent, and recomputed when informed of the
        // remote session endpoint state.
        self.remote_incoming_window -= 1;

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
        // Currently the sender cannot actively dispose any message
        // because the sender doesn't have access to the delivery_id

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

    async fn send_end<W>(
        &mut self,
        writer: &mut W,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame, Error = mpsc::error::SendError<SessionFrame>> + Send + Unpin,
    {
        match self.local_state {
            SessionState::Mapped => match error.is_some() {
                true => self.local_state = SessionState::Discarding,
                false => self.local_state = SessionState::EndSent,
            },
            SessionState::EndReceived => self.local_state = SessionState::Unmapped,
            _ => return Err(AmqpError::IllegalState.into()),
        }

        let frame = SessionFrame::new(self.outgoing_channel, SessionFrameBody::End(End { error }));
        writer
            .send(frame)
            .await
            // The receiving half must have dropped, and thus the `Connection`
            // event loop has stopped. It should be treated as an io error
            .map_err(|e| Self::Error::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
        Ok(())
    }
}
