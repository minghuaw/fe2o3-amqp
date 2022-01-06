mod frame;
use std::{sync::Arc, collections::BTreeMap, marker::PhantomData};

use async_trait::async_trait;
use bytes::Bytes;
use fe2o3_amqp_types::{
    definitions::{AmqpError, Handle, ReceiverSettleMode, Role, SenderSettleMode, self, DeliveryTag, MessageFormat},
    messaging::{DeliveryState, Source, Target}, primitives::Symbol, performatives::{Attach, Disposition, Detach, Transfer},
};
pub use frame::*;
pub mod builder;
pub mod delivery;
mod error;
pub mod receiver;
pub mod sender;
pub mod state;

pub use error::Error;

use futures_util::{Sink, Stream, SinkExt};
pub use receiver::Receiver;
pub use sender::Sender;
use tokio::sync::{mpsc, RwLock, oneshot};

use crate::{
    endpoint::{self, LinkFlow, Settlement},
    util::{Producer, Consumer},
    link::{self, state::SenderPermit, delivery::UnsettledMessage}
};

use self::state::{LinkFlowState, UnsettledMap, LinkState};

pub mod type_state {
    pub struct Attached {}

    pub struct Detached {}
}

pub mod role {
    use fe2o3_amqp_types::definitions::Role;

    /// Type state for link::builder::Builder
    pub struct Sender {}

    /// Type state for link::builder::Builder
    pub struct Receiver {}

    pub trait IntoRole {
        fn into_role() -> Role;
    }

    impl IntoRole for Sender {
        fn into_role() -> Role {
            Role::Sender
        }
    }

    impl IntoRole for Receiver {
        fn into_role() -> Role {
            Role::Receiver
        }
    }
}


/// Manages the link state
/// 
/// # Type Parameters
/// 
/// R: role
/// 
/// F: link flow state
pub struct Link<R, F> {
    pub(crate) role: PhantomData<R>,

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
    // pub(crate) flow_state: Consumer<Arc<LinkFlowState>>,
    pub(crate) flow_state: F,
    pub(crate) unsettled: Arc<RwLock<UnsettledMap>>,
}

#[async_trait]
impl<R, F> endpoint::Link for Link<R, F> 
where 
    R: role::IntoRole + Send,
    F: AsRef<LinkFlowState> + Send + Sync,
{
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
                        .map(|(key, val)| (DeliveryTag::from(*key), val.state().clone()))
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
        let initial_delivery_count = Some(self.flow_state.as_ref().initial_delivery_count().await);
        let properties = self.flow_state.as_ref().properties().await;

        let attach = Attach {
            name: self.name.clone(),
            handle: handle,
            role: R::into_role(),
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
impl endpoint::SenderLink for Link<role::Sender, Consumer<Arc<LinkFlowState>>> {
    async fn send_transfer<W>(
        &mut self,
        writer: &mut W,
        payload: Bytes,
        message_format: MessageFormat,
        settled: Option<bool>,
        batchable: bool,
    ) -> Result<Settlement, <Self as endpoint::Link>::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
    {
        use crate::util::Consume;

        println!(">>> Debug: SenderLink::send_transfer");

        match self.flow_state.consume(1).await {
            SenderPermit::Send => { }, // There is enough credit to send
            SenderPermit::Drain => {
                // Drain is set
                todo!()
            }
        }

        // Check message size
        if (self.max_message_size == 0) || (payload.len() as u64 <= self.max_message_size) {
            let handle = self
                .output_handle
                .clone()
                .ok_or_else(|| AmqpError::IllegalState)?;

            let delivery_tag = self.flow_state.state().delivery_count().await.to_be_bytes();

            // TODO: Expose API to allow user to set this when the mode is MIXED?
            let settled = match self.snd_settle_mode {
                SenderSettleMode::Settled => true,
                SenderSettleMode::Unsettled => false,
                // If not set on the first (or only) transfer for a (multi-transfer)
                // delivery, then the settled flag MUST be interpreted as being false.
                SenderSettleMode::Mixed => settled.unwrap_or_else(|| false),
            };

            // TODO: Expose API for resuming link?
            let state: Option<DeliveryState> = None;
            let resume = false;

            let transfer = Transfer {
                handle,
                delivery_id: None, // This will be set by the session
                delivery_tag: Some(DeliveryTag::from(delivery_tag)),
                message_format: Some(message_format),
                settled: Some(settled), // Having this always set in first frame helps debugging
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
                payload: payload.clone(), // Clone should be very cheap for Bytes
            };
            writer
                .send(frame)
                .await
                .map_err(|_| link::Error::AmqpError {
                    condition: AmqpError::IllegalState,
                    description: Some("Session is already dropped".to_string()),
                })?;

            match settled {
                true => Ok(Settlement::Settled),
                // If not set on the first (or only) transfer for a (multi-transfer)
                // delivery, then the settled flag MUST be interpreted as being false.
                false => {
                    let (tx, rx) = oneshot::channel();
                    let unsettled = UnsettledMessage::new(payload, tx);
                    {
                        let mut guard = self.unsettled.write().await;
                        guard.insert(delivery_tag, unsettled);
                    }

                    Ok(Settlement::Unsettled {
                        delivery_tag,
                        outcome: rx,
                    })
                }
            }
        } else {
            // Need multiple transfers
            todo!()
        }
    }
}


// pub struct LinkHandle {
//     tx: mpsc::Sender<LinkIncomingItem>,
//     flow_state: Producer<Arc<LinkFlowState>>,
//     unsettled: Arc<RwLock<UnsettledMap>>,
//     // This is not expect to change
//     pub(crate) receiver_settle_mode: ReceiverSettleMode,
// }

/// TODO: How would this be changed when switched to ReceiverLink
pub enum LinkHandle {
    Sender {
        tx: mpsc::Sender<LinkIncomingItem>,
        // This should be wrapped inside a Producer because the SenderLink
        // needs to consume link credit from LinkFlowState
        flow_state: Producer<Arc<LinkFlowState>>,
        unsettled: Arc<RwLock<UnsettledMap>>,
        receiver_settle_mode: ReceiverSettleMode,
    },
    Receiver {
        tx: mpsc::Sender<LinkIncomingItem>,
        flow_state: Arc<LinkFlowState>,
        unsettled: Arc<RwLock<UnsettledMap>>,
        receiver_settle_mode: ReceiverSettleMode,
    }
}

impl LinkHandle {
    pub(crate) async fn send(
        &mut self,
        frame: LinkFrame,
    ) -> Result<(), mpsc::error::SendError<LinkFrame>> {
        match self {
            LinkHandle::Sender {tx, ..} => {
                tx.send(frame).await
            }
            LinkHandle::Receiver {tx, ..} => {
                tx.send(frame).await
            }
        }
    }

    pub(crate) async fn on_incoming_flow(
        &mut self,
        flow: LinkFlow,
        output_handle: Handle,
    ) -> Option<LinkFlow> {
        match self {
            LinkHandle::Sender{flow_state, ..} => {
                flow_state.on_incoming_flow(flow, output_handle).await
            }
            LinkHandle::Receiver{flow_state, ..} => {
                flow_state.on_incoming_flow(flow, output_handle).await
            }
        }

    }

    pub(crate) async fn on_incoming_disposition(
        &mut self,
        role: Role,
        is_settled: bool,
        state: &Option<DeliveryState>,
        // Disposition only contains the delivery ids, which are assigned by the
        // sessions
        delivery_tag: [u8; 4],
    ) -> bool {
        match self {
            LinkHandle::Sender {
                unsettled, 
                receiver_settle_mode, 
                ..} => {
                // TODO: verfify role?
                if is_settled {
                    // TODO: Reply with disposition?
                    // Upon receiving the updated delivery state from the receiver, the sender will, if it has not already spontaneously
                    // attained a terminal state (e.g., through the expiry of the TTL at the sender), update its view of the state and
                    // communicate this back to the sending application.
                    {
                        let mut guard = unsettled.write().await;
                        if let Some(unsettled) = guard.remove(&delivery_tag) {
                            // Since we are settling (ie. forgetting) this message, we don't care whether the 
                            // receiving end is alive or not
                            let _ = unsettled.settle_with_state(state.clone());
                        }
                    }
                    match receiver_settle_mode {
                        ReceiverSettleMode::First => {
                            // The receiver will spontaneously settle all incoming transfers.
                            false
                        }
                        ReceiverSettleMode::Second => {
                            // The receiver will only settle after sending the disposition to
                            // the sender and receiving a disposition indicating settlement of the
                            // delivery from the sender.
                            true
                        }
                    }
                } else {
                    {
                        let mut guard = unsettled.write().await;
                        if let Some(unsettled) = guard.get_mut(&delivery_tag) {
                            if let Some(state) = state {
                                *unsettled.state_mut() = state.clone();
                            }
                        }
                    }
                    false
                }
            },
            LinkHandle::Receiver {unsettled, ..} => {
                todo!()
            }
        }
    }
}

pub(crate) async fn do_attach<L, W, R>(
    link: &mut L,
    writer: &mut W,
    reader: &mut R,
) -> Result<(), Error>
where
    L: endpoint::Link<Error = Error>,
    W: Sink<LinkFrame, Error = mpsc::error::SendError<LinkFrame>> + Send + Unpin,
    R: Stream<Item = LinkFrame> + Send + Unpin,
{
    use futures_util::StreamExt;

    // Send an Attach frame
    endpoint::Link::send_attach(link, writer).await?;

    // Wait for an Attach frame
    let frame = reader.next().await.ok_or_else(|| Error::AmqpError {
        condition: AmqpError::IllegalState,
        description: Some("Expecting remote attach frame".to_string()),
    })?;
    let remote_attach = match frame {
        LinkFrame::Attach(attach) => attach,
        // TODO: how to handle this?
        _ => {
            return Err(Error::AmqpError {
                condition: AmqpError::IllegalState,
                description: Some("Expecting remote attach frame".to_string()),
            })
        }
    };

    // Note that if the application chooses not to create a terminus,
    // the session endpoint will still create a link endpoint and issue
    // an attach indicating that the link endpoint has no associated
    // local terminus. In this case, the session endpoint MUST immediately
    // detach the newly created link endpoint.
    match remote_attach.target.is_some() {
        true => {
            if let Err(_) = link.on_incoming_attach(remote_attach).await {
                // Should any error happen handling remote
                todo!()
            }
        }
        false => {
            // If no target is supplied with the remote attach frame,
            // an immediate detach should be expected
            expect_detach_then_detach(link, writer, reader).await?;
        }
    }

    Ok(())
}

pub(crate) async fn expect_detach_then_detach<L, W, R>(
    link: &mut L,
    writer: &mut W,
    reader: &mut R,
) -> Result<(), Error>
where
    L: endpoint::Link<Error = Error>,
    W: Sink<LinkFrame, Error = mpsc::error::SendError<LinkFrame>> + Send + Unpin,
    R: Stream<Item = LinkFrame> + Send + Unpin,
{
    use futures_util::StreamExt;

    let frame = reader.next().await.ok_or_else(|| Error::AmqpError {
        condition: AmqpError::IllegalState,
        description: Some("Expecting remote detach frame".to_string()),
    })?;
    let _remote_detach = match frame {
        LinkFrame::Detach(detach) => detach,
        _ => {
            return Err(Error::AmqpError {
                condition: AmqpError::IllegalState,
                description: Some("Expecting remote detach frame".to_string()),
            })
        }
    };

    link.send_detach(writer, false, None).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{link::state::LinkFlowStateInner, util::Constant};

    #[tokio::test]
    async fn test_producer_notify() {
        use std::sync::Arc;
        use tokio::sync::Notify;
        use tokio::sync::RwLock;

        use super::*;
        use crate::util::{Produce, Producer};

        let notifier = Arc::new(Notify::new());
        let state = LinkFlowState::Sender(RwLock::new(LinkFlowStateInner {
            initial_delivery_count: Constant::new(0),
            delivery_count: 0,
            link_credit: 0,
            avaiable: 0,
            drain: false,
            properties: None,
        }));
        let mut producer = Producer::new(notifier.clone(), Arc::new(state));
        let notified = notifier.notified();

        let handle = tokio::spawn(async move {
            let item = (LinkFlow::default(), Handle::from(0));
            producer.produce(item).await;
        });

        notified.await;
        println!("wait passed");

        handle.await.unwrap();
    }
}
