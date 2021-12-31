use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use bytes::BytesMut;
use fe2o3_amqp_types::{
    definitions::{
        self, AmqpError, DeliveryTag, Handle, MessageFormat, ReceiverSettleMode, Role,
        SenderSettleMode,
    },
    messaging::{DeliveryState, Source, Target},
    performatives::{Attach, Detach, Disposition, Transfer},
    primitives::Symbol,
};
use futures_util::{Sink, SinkExt};
use tokio::sync::{mpsc, RwLock, oneshot};

use crate::{
    endpoint::{self, Settlement},
    util::Consumer,
};

use super::{LinkFlowState, LinkFrame, LinkState, UnsettledMap, delivery::UnsettledDelivery};
use crate::link;

/// Manages the link state
pub struct SenderLink {
    pub(crate) local_state: LinkState,

    pub(crate) name: String,

    pub(crate) output_handle: Option<Handle>, // local handle
    pub(crate) input_handle: Option<Handle>,  // remote handle

    /// The `Sender` will manage whether to wait for incoming disposition
    pub(crate) snd_settle_mode: SenderSettleMode,
    pub(crate) rcv_settle_mode: ReceiverSettleMode,

    pub(crate) source: Option<Source>, // TODO: Option?
    pub(crate) target: Option<Target>, // TODO: Option?

    /// If zero, the max size is not set.
    /// If zero, the attach frame should treated is None
    pub(crate) max_message_size: u64,

    // capabilities
    pub(crate) offered_capabilities: Option<Vec<Symbol>>,
    pub(crate) desired_capabilities: Option<Vec<Symbol>>,

    // See Section 2.6.7 Flow Control
    // pub(crate) delivery_count: SequenceNo, // TODO: the first value is the initial_delivery_count?
    // pub(crate) properties: Option<Fields>,
    pub(crate) flow_state: Consumer<Arc<LinkFlowState>>,
    pub(crate) unsettled: Arc<RwLock<UnsettledMap>>,
}

impl SenderLink {
    // pub fn new() -> Self {
    //     todo!()
    // }
}

#[async_trait]
impl endpoint::Link for SenderLink {
    type DetachError = definitions::Error;
    type Error = link::Error;

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error> {
        println!(">>> Debug: SenderLink::on_incoming_attach");
        match self.local_state {
            LinkState::AttachSent => self.local_state = LinkState::Attached,
            LinkState::Unattached => self.local_state = LinkState::AttachReceived,
            LinkState::Detached => {
                // remote peer is attempting to re-attach
                self.local_state = LinkState::AttachReceived
            }
            _ => return Err(AmqpError::IllegalState.into()),
        };

        self.input_handle = Some(attach.handle);

        // When resuming a link, it is possible that the properties of the source and target have changed while the link
        // was suspended. When this happens, the termini properties communicated in the source and target fields of the
        // attach frames could be in conflict. In this case, the sender is considered to hold the authoritative version of the
        // **the receiver is considered to hold the authoritative version of the target properties**.
        self.target = attach.target;

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

    /// Closing or not isn't taken care of here but outside
    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::DetachError> {
        println!(">>> Debug: SenderLink::on_incoming_detach");

        match self.local_state {
            LinkState::Attached => self.local_state = LinkState::DetachReceived,
            LinkState::DetachSent => self.local_state = LinkState::Detached,
            _ => {
                return Err(definitions::Error::new(
                    AmqpError::IllegalState,
                    Some("Illegal local state".into()),
                    None,
                )
                .into())
            }
        };

        // Receiving detach forwarded by session means it's ready to drop the output handle
        let _ = self.output_handle.take();

        if let Some(err) = detach.error {
            return Err(err.into());
        }
        Ok(())
    }

    async fn send_attach<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame, Error = mpsc::error::SendError<LinkFrame>> + Send + Unpin,
    {
        println!(">>> Debug: SenderLink::send_attach");
        println!(">>> Debug: SenderLink.local_state: {:?}", &self.local_state);

        // Create Attach frame
        let handle = match &self.output_handle {
            Some(h) => h.clone(),
            None => {
                return Err(link::Error::AmqpError {
                    condition: AmqpError::InvalidField,
                    description: Some("Output handle is None".into()),
                })
            }
        };
        let unsettled: Option<BTreeMap<DeliveryTag, DeliveryState>> = {
            let guard = self.unsettled.read().await;
            match guard.len() {
                0 => None,
                _ => Some(
                    guard
                        .iter()
                        .map(|(key, val)| {
                            (DeliveryTag::from(key.to_be_bytes()), val.state().clone())
                        })
                        .collect(),
                ),
            }
        };
        // let unsettled: Option<UnsettledMap> = match self.unsettled.read().await.len() {
        //     0 => None,
        //     _ => Some(self.unsettled.clone()),
        // };
        let max_message_size = match self.max_message_size {
            0 => None,
            val @ _ => Some(val as u64),
        };
        let initial_delivery_count = Some(self.flow_state.state().initial_delivery_count().await);
        let properties = self.flow_state.state().properties().await;

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
            initial_delivery_count,

            max_message_size,
            offered_capabilities: self.offered_capabilities.clone(),
            desired_capabilities: self.desired_capabilities.clone(),
            properties,
        };
        let frame = LinkFrame::Attach(attach);

        match self.local_state {
            LinkState::Unattached
            | LinkState::Detached // May attempt to re-attach
            | LinkState::DetachSent => {
                writer.send(frame).await
                    .map_err(|e| Self::Error::from(e))?;
                self.local_state = LinkState::AttachSent
            }
            LinkState::AttachReceived => {
                writer.send(frame).await
                    .map_err(|e| Self::Error::from(e))?;
                self.local_state = LinkState::Attached
            }
            _ => return Err(AmqpError::IllegalState.into()),
        }

        Ok(())
    }

    async fn send_flow<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
    {
        todo!()
    }

    async fn send_disposition<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
    {
        todo!()
    }

    async fn send_detach<W>(
        &mut self,
        writer: &mut W,
        closed: bool,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame, Error = mpsc::error::SendError<LinkFrame>> + Send + Unpin,
    {
        println!(">>> Debug: SenderLink::send_detach");
        println!(">>> Debug: SenderLink local_state: {:?}", &self.local_state);

        // Detach may fail and may try re-attach
        // The local output handle is not removed from the session
        // until `deallocate_link`. The outgoing detach will not remove
        // local handle from session
        match &self.output_handle {
            Some(handle) => {
                match self.local_state {
                    LinkState::Attached => self.local_state = LinkState::DetachSent,
                    LinkState::DetachReceived => self.local_state = LinkState::Detached,
                    _ => return Err(AmqpError::IllegalState.into()),
                };

                let detach = Detach {
                    handle: handle.clone(),
                    closed,
                    error,
                };
                writer.send(LinkFrame::Detach(detach)).await.map_err(|_| {
                    link::Error::AmqpError {
                        condition: AmqpError::IllegalState,
                        description: Some("Session is already dropped".to_string()),
                    }
                })?;
            }
            None => {
                return Err(link::Error::AmqpError {
                    condition: AmqpError::IllegalState,
                    description: Some("Link is already detached".to_string()),
                })
            }
        }

        Ok(())
    }
}

#[async_trait]
impl endpoint::SenderLink for SenderLink {
    async fn send_transfer<W>(
        &mut self,
        writer: &mut W,
        payload: BytesMut,
        message_format: MessageFormat,
        settled: Option<bool>,
        batchable: bool,
    ) -> Result<Settlement, <Self as endpoint::Link>::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
    {
        use crate::util::Consume;

        println!(">>> Debug: SenderLink::send_transfer");

        // Whether drain flag is set
        if self.flow_state.state().drain().await {
            // println!(">>> Debug: SenderLink::send_transfer drain");
            // return Ok(Settlement::Settled);
            todo!()
        }

        // Consume link credit
        self.flow_state.consume(1).await;

        // Check message size
        if (self.max_message_size == 0) || (payload.len() as u64 <= self.max_message_size) {
            let handle = self
                .output_handle
                .clone()
                .ok_or_else(|| AmqpError::IllegalState)?;

            let delivery_count = self.flow_state.state().delivery_count().await;
            let delivery_tag: [u8; 4] = delivery_count.to_be_bytes();

            // TODO: Expose API to allow user to set this when the mode is MIXED?
            let settled = match self.snd_settle_mode {
                SenderSettleMode::Settled => Some(true),
                SenderSettleMode::Unsettled => {
                    // TODO: add to unsettled map
                    todo!()
                    // Some(false)
                }
                SenderSettleMode::Mixed => {
                    // TODO: conditionally add to the unsettled map
                    todo!()
                    // settled
                }
            };

            // TODO: Expose API for resuming link?
            let state: Option<DeliveryState> = None;
            let resume = false;

            let transfer = Transfer {
                handle,
                delivery_id: None, // This will be set by the session
                delivery_tag: Some(DeliveryTag::from(delivery_tag)),
                message_format: Some(message_format),
                settled,
                more: false,
                // If not set, this value is defaulted to the value negotiated
                // on link attach.
                rcv_settle_mode: None,
                state,
                resume,
                aborted: false,
                batchable,
            };

            let frame = LinkFrame::Transfer {
                performative: transfer,
                payload,
            };
            writer
                .send(frame)
                .await
                .map_err(|_| link::Error::AmqpError {
                    condition: AmqpError::IllegalState,
                    description: Some("Session is already dropped".to_string()),
                })?;

            match settled {
                Some(true) => Ok(Settlement::Settled),
                // If not set on the first (or only) transfer for a (multi-transfer)
                // delivery, then the settled flag MUST be interpreted as being false.
                Some(false) | None => {
                    let (tx, rx) = oneshot::channel();
                    let unsettled = UnsettledDelivery::new(tx);
                    {
                        let mut guard = self.unsettled.write().await;
                        guard.insert(delivery_count, unsettled);
                    }

                    Ok(Settlement::Unsettled(rx))
                }
            }
        } else {
            // Need multiple transfers
            todo!()
        }
    }
}