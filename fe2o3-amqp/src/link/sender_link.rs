use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{
        DeliveryTag, Fields, Handle, ReceiverSettleMode, Role, SenderSettleMode, SequenceNo,
    },
    messaging::{DeliveryState, Source, Target},
    performatives::{Attach, Detach, Disposition, Flow},
    primitives::Symbol,
};
use futures_util::{Sink, SinkExt};
use tokio::sync::mpsc;

use crate::{endpoint, error::EngineError, util::Constant};

use super::{LinkFlowState, LinkFrame, LinkState};

/// Manages the link state
pub struct SenderLink {
    pub(crate) local_state: LinkState,

    pub(crate) name: String,

    pub(crate) output_handle: Option<Handle>, // local handle
    pub(crate) input_handle: Option<Handle>,  // remote handle

    pub(crate) snd_settle_mode: SenderSettleMode,
    pub(crate) rcv_settle_mode: ReceiverSettleMode,
    pub(crate) source: Option<Source>, // TODO: Option?
    pub(crate) target: Option<Target>, // TODO: Option?

    pub(crate) unsettled: BTreeMap<DeliveryTag, DeliveryState>,

    /// If zero, the max size is not set.
    /// If zero, the attach frame should treated is None
    pub(crate) max_message_size: u64,
    
    // capabilities
    pub(crate) offered_capabilities: Option<Vec<Symbol>>,
    pub(crate) desired_capabilities: Option<Vec<Symbol>>,
    
    // See Section 2.6.7 Flow Control
    // pub(crate) delivery_count: SequenceNo, // TODO: the first value is the initial_delivery_count?
    // pub(crate) properties: Option<Fields>,
    pub(crate) flow_state: Arc<LinkFlowState>,
}

impl SenderLink {
    // pub fn new() -> Self {
    //     todo!()
    // }
}

#[async_trait]
impl endpoint::Link for SenderLink {
    type Error = EngineError;

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error> {
        println!(">>> Debug: SenderLink::on_incoming_attach");

        // Note that if the application chooses not to create a terminus, 
        // the session endpoint will still create a link endpoint and issue
        // an attach indicating that the link endpoint has no associated 
        // local terminus. In this case, the session endpoint MUST immediately 
        // detach the newly created link endpoint.
        if attach.target.is_none() || attach.source.is_none() {
            return Err(EngineError::LinkAttachRefused) // TODO: this should then be handled outside
        }

        self.input_handle = Some(attach.handle);

        if self.target.is_none() {
            self.target = attach.target; // TODO: ???
        }

        // set max message size
        let remote_max_msg_size = attach.max_message_size.unwrap_or_else(|| 0);
        if remote_max_msg_size < self.max_message_size {
            self.max_message_size = remote_max_msg_size;
        }

        Ok(())
    }

    // async fn on_incoming_flow(&mut self, flow: Flow) -> Result<(), Self::Error> {
    //     todo!()
    // }

    // Only the receiver is supposed to receive incoming Transfer frame

    async fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_attach<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
        W::Error: Into<Self::Error>,
    {
        // Create Attach frame
        let handle = match &self.output_handle {
            Some(h) => h.clone(),
            None => return Err(EngineError::Message("Output handle is not assigned")),
        };
        let unsettled = match self.unsettled.len() {
            0 => None,
            _ => Some(self.unsettled.clone()),
        };
        let max_message_size = match self.max_message_size {
            0 => None,
            val @ _ => Some(val as u64),
        };

        let attach = Attach {
            name: self.name.clone(),
            handle: handle,
            role: Role::Sender,
            snd_settle_mode: self.snd_settle_mode.clone(),
            rcv_settle_mode: self.rcv_settle_mode.clone(),
            source: self.source.clone(),
            target: self.target.clone(),
            unsettled,
            incomplete_unsettled: false, // TODO: try send once and then retry if frame size too large?

            /// This MUST NOT be null if role is sender,
            /// and it is ignored if the role is receiver.
            /// See subsection 2.6.7.
            initial_delivery_count: Some(*self.flow_state.initial_delivery_count()),

            max_message_size,
            offered_capabilities: self.offered_capabilities.clone(),
            desired_capabilities: self.desired_capabilities.clone(),
            properties: self.flow_state.properties().await,
        };
        let frame = LinkFrame::Attach(attach);

        match self.local_state {
            LinkState::Unattached => {
                writer.send(frame).await.map_err(Into::into)?;
                self.local_state = LinkState::AttachSent
            }
            LinkState::AttachReceived => {
                writer.send(frame).await.map_err(Into::into)?;
                self.local_state = LinkState::Attached
            }
            _ => return Err(EngineError::Message("Wrong LinkState")),
        }

        Ok(())
    }

    async fn send_flow<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
        W::Error: Into<Self::Error>,
    {
        todo!()
    }

    async fn send_disposition<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
        W::Error: Into<Self::Error>,
    {
        todo!()
    }

    async fn send_detach<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
        W::Error: Into<Self::Error>,
    {
        todo!()
    }
}

#[async_trait]
impl endpoint::SenderLink for SenderLink {
    async fn send_transfer<W>(
        &mut self,
        writer: &mut W,
    ) -> Result<(), <Self as endpoint::Link>::Error> 
    where   
        W: Sink<LinkFrame> + Send + Unpin,
        W::Error: Into<Self::Error>,
    {
        todo!()
    }
}
