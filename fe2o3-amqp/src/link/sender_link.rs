use std::collections::BTreeMap;

use async_trait::async_trait;
use fe2o3_amqp_types::{definitions::{DeliveryTag, Fields, Handle, ReceiverSettleMode, Role, SenderSettleMode, SequenceNo}, messaging::{DeliveryState, Source, Target}, performatives::{Attach, Detach, Disposition, Flow}, primitives::Symbol};
use tokio::sync::mpsc;

use crate::{endpoint, error::EngineError};

use super::{LinkFrame, LinkState};

/// Manages the link state
pub struct SenderLink {
    local_state: LinkState,

    name: String,

    output_handle: Option<Handle>, // local handle
    input_handle: Option<Handle>, // remote handle

    snd_settle_mode: SenderSettleMode,
    rcv_settle_mode: ReceiverSettleMode,
    source: Option<Source>, // TODO: Option?
    target: Option<Target>, // TODO: Option?

    unsettled: BTreeMap<DeliveryTag, DeliveryState>,
    
    delivery_count: SequenceNo, // TODO: the first value is the initial_delivery_count?
    
    /// If zero, the max size is not set. 
    /// If zero, the attach frame should treated is None
    max_message_size: usize, 

    // capabilities
    offered_capabilities: Option<Vec<Symbol>>,
    desired_capabilities: Option<Vec<Symbol>>,
    properties: Option<Fields>,
}

impl SenderLink {
    pub fn new() -> Self {
        todo!()
    }
}

#[async_trait]
impl endpoint::Link for SenderLink {
    type Error = EngineError;
    
    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_flow(&mut self, flow: Flow) -> Result<(), Self::Error> {
        todo!()
    }

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

    async fn send_attach(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::Error> {
        // Create Attach frame
        let handle = match &self.output_handle {
            Some(h) => h.clone(),
            None => return Err(EngineError::Message("Output handle is not assigned"))
        };
        let unsettled = match self.unsettled.len() {
            0 => None,
            _ => Some(self.unsettled.clone())
        };
        let max_message_size = match self.max_message_size {
            0 => None,
            val @ _ => Some(val as u64)
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
            initial_delivery_count: Some(self.delivery_count.clone()),

            max_message_size,
            offered_capabilities: self.offered_capabilities.clone(),
            desired_capabilities: self.desired_capabilities.clone(),
            properties: self.properties.clone(),
        };
        let frame = LinkFrame::Attach(attach);
        
        match self.local_state {
            LinkState::Unattached => {
                writer.send(frame).await?;
                self.local_state = LinkState::AttachSent
            },
            LinkState::AttachReceived => {
                writer.send(frame).await?;
                self.local_state = LinkState::Attached
            },
            _ => return Err(EngineError::Message("Wrong LinkState"))
        }

        Ok(())
    }

    async fn send_flow(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_disposition(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_detach(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

#[async_trait]
impl endpoint::SenderLink for SenderLink {
    async fn send_transfer(&mut self, writer: &mut mpsc::Sender<LinkFrame>) -> Result<(), <Self as endpoint::Link>::Error> {
        todo!()
    }
}
