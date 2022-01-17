mod frame;
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use bytes::Buf;
use fe2o3_amqp_types::{
    definitions::{
        self, AmqpError, DeliveryNumber, DeliveryTag, Handle, MessageFormat, ReceiverSettleMode,
        Role, SenderSettleMode, SessionError,
    },
    messaging::{Accepted, DeliveryState, Message, Received, Source, Target},
    performatives::{Attach, Detach, Disposition, Transfer},
    primitives::Symbol,
};
pub use frame::*;
pub mod builder;
pub mod delivery;
mod error;
pub mod receiver;
mod receiver_link;
pub mod sender;
mod sender_link;
pub mod state;

pub use error::Error;

use futures_util::{Sink, SinkExt, Stream};
pub use receiver::Receiver;
pub use sender::Sender;
use serde_amqp::from_reader;
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::{
    endpoint::{self, LinkFlow, ReceiverLink, Settlement},
    link::{self, delivery::UnsettledMessage, state::SenderPermit},
    session,
    util::{Consumer, Producer},
    Payload,
};

use self::{
    delivery::Delivery,
    state::{LinkFlowState, LinkState, UnsettledMap},
};

type SenderFlowState = Consumer<Arc<LinkFlowState<role::Sender>>>;
type ReceiverFlowState = Arc<LinkFlowState<role::Receiver>>;

pub mod type_state {
    #[derive(Debug)]
    pub struct Attached {}

    #[derive(Debug)]
    pub struct Detached {}
}

pub mod role {
    use fe2o3_amqp_types::definitions::Role;

    /// Type state for link::builder::Builder
    #[derive(Debug)]
    pub struct Sender {}

    /// Type state for link::builder::Builder
    #[derive(Debug)]
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
#[derive(Debug)]
pub struct Link<R, F, M> {
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
    pub(crate) unsettled: Arc<RwLock<UnsettledMap<M>>>,
}

#[async_trait]
// impl endpoint::Link for Link<role::Sender, Consumer<Arc<LinkFlowState>>> {
impl<R, F, M> endpoint::Link for Link<R, F, M>
where
    R: role::IntoRole + Send + Sync,
    F: AsRef<LinkFlowState<R>> + Send + Sync,
    M: AsRef<DeliveryState> + AsMut<DeliveryState> + Send + Sync,
{
    type DetachError = definitions::Error;
    type Error = link::Error;

    async fn on_incoming_attach(&mut self, remote_attach: Attach) -> Result<(), Self::Error> {
        println!(">>> Debug: Link<{:?}>::on_incoming_attach", R::into_role());
        match self.local_state {
            LinkState::AttachSent => self.local_state = LinkState::Attached,
            LinkState::Unattached => self.local_state = LinkState::AttachReceived,
            LinkState::Detached => {
                // remote peer is attempting to re-attach
                self.local_state = LinkState::AttachReceived
            }
            _ => return Err(AmqpError::IllegalState.into()),
        };

        self.input_handle = Some(remote_attach.handle);

        // When resuming a link, it is possible that the properties of the source and target have changed while the link
        // was suspended. When this happens, the termini properties communicated in the source and target fields of the
        // attach frames could be in conflict.
        match remote_attach.role {
            // Remote attach is from sender
            Role::Sender => {
                // In this case, the sender is considered to hold the authoritative version of the
                self.source = remote_attach.source;
                // The receiver SHOULD respect the sender’s desired settlement mode if the sender
                // initiates the attach exchange and the receiver supports the desired mode.
                self.snd_settle_mode = remote_attach.snd_settle_mode;

                // The delivery-count is initialized by the sender when a link endpoint is
                // created, and is incre- mented whenever a message is sent
                println!("{:?}", remote_attach.initial_delivery_count);
                let initial_delivery_count = match remote_attach.initial_delivery_count {
                    Some(val) => val,
                    None => return Err(AmqpError::NotAllowed.into()),
                };
                self.flow_state
                    .as_ref()
                    .initial_delivery_count_mut(|_| initial_delivery_count)
                    .await;
                self.flow_state
                    .as_ref()
                    .delivery_count_mut(|_| initial_delivery_count)
                    .await;
            }
            // Remote attach is from receiver
            Role::Receiver => {
                // **the receiver is considered to hold the authoritative version of the target properties**.
                self.target = remote_attach.target;
                // The sender SHOULD respect the receiver’s desired settlement mode if the receiver
                // initiates the attach exchange and the sender supports the desired mode
                self.rcv_settle_mode = remote_attach.rcv_settle_mode;
            }
        }

        // set max message size
        // If this field is zero or unset, there is no maximum size imposed by the link endpoint.
        let remote_max_msg_size = remote_attach.max_message_size.unwrap_or_else(|| 0);
        if remote_max_msg_size < self.max_message_size {
            self.max_message_size = remote_max_msg_size;
        }

        // TODO: what to do with the unattached

        Ok(())
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
        println!(">>> Debug: Link::send_attach");
        println!(">>> Debug: Link.local_state: {:?}", &self.local_state);

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
                        .map(|(key, val)| (key.clone(), val.as_ref().clone()))
                        .collect(),
                ),
            }
        };

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

    // async fn send_flow<W>(&mut self, writer: &mut W, echo: bool) -> Result<(), Self::Error>
    // where
    //     W: Sink<LinkFlow> + Send + Unpin,
    // {
    //     let handle = self.output_handle.clone()
    //         .ok_or_else(|| Error::AmqpError {
    //             condition: AmqpError::IllegalState,
    //             description: Some("Link is not attached".into())
    //         })?;

    //     let flow = {
    //         let reader = self.flow_state.as_ref().lock.read().await;
    //         LinkFlow {
    //             handle,
    //             delivery_count: Some(reader.delivery_count.clone()),
    //             link_credit: Some(reader.link_credit.clone()),
    //             available: Some(reader.available.clone()),
    //             drain: reader.drain,
    //             echo,
    //             properties: reader.properties.clone()
    //         }
    //     };

    //     writer.send(flow).await
    //         .map_err(|_| Error::AmqpError {
    //             condition: AmqpError::IllegalState,
    //             description: Some("Link is not attached".into())
    //         })
    // }

    // /// This doesnt remove the delivery from the unsettled map until the outgoing disposition
    // /// is processed by the session loop because disposition doesn't include any info on
    // /// delivery_tag
    // async fn send_disposition<W>(&mut self, writer: &mut W, disposition: Disposition) -> Result<(), Self::Error>
    // where
    //     W: Sink<LinkFrame> + Send + Unpin,
    // {
    //     let frame = LinkFrame::Disposition(disposition);
    //     writer.send(frame).await
    //         .map_err(|_| link::Error::AmqpError {
    //             condition: AmqpError::IllegalState,
    //             description: Some("Session is already dropped".to_string()),
    //         })
    // }

    async fn send_detach<W>(
        &mut self,
        writer: &mut W,
        closed: bool,
        error: Option<Self::DetachError>,
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

/// TODO: How would this be changed when switched to ReceiverLink
pub enum LinkHandle {
    Sender {
        tx: mpsc::Sender<LinkIncomingItem>,
        // This should be wrapped inside a Producer because the SenderLink
        // needs to consume link credit from LinkFlowState
        flow_state: Producer<Arc<LinkFlowState<role::Sender>>>,
        unsettled: Arc<RwLock<UnsettledMap<UnsettledMessage>>>,
        receiver_settle_mode: ReceiverSettleMode,
    },
    Receiver {
        tx: mpsc::Sender<LinkIncomingItem>,
        flow_state: ReceiverFlowState,
        unsettled: Arc<RwLock<UnsettledMap<DeliveryState>>>,
        receiver_settle_mode: ReceiverSettleMode,
        more: bool,
    },
}

impl LinkHandle {
    pub(crate) async fn send(
        &mut self,
        frame: LinkFrame,
    ) -> Result<(), mpsc::error::SendError<LinkFrame>> {
        match self {
            LinkHandle::Sender { tx, .. } => tx.send(frame).await,
            LinkHandle::Receiver { tx, .. } => tx.send(frame).await,
        }
    }

    pub(crate) async fn on_incoming_flow(
        &mut self,
        flow: LinkFlow,
        output_handle: Handle,
    ) -> Option<LinkFlow> {
        match self {
            LinkHandle::Sender { flow_state, .. } => {
                flow_state.on_incoming_flow(flow, output_handle).await
            }
            LinkHandle::Receiver { flow_state, .. } => {
                flow_state.on_incoming_flow(flow, output_handle).await
            }
        }
    }

    /// Returns whether an echo is needed
    pub(crate) async fn on_incoming_disposition(
        &mut self,
        _role: Role, // Is a role check necessary?
        settled: bool,
        state: Option<DeliveryState>,
        // Disposition only contains the delivery ids, which are assigned by the
        // sessions
        delivery_tag: DeliveryTag,
    ) -> bool {
        match self {
            LinkHandle::Sender {
                unsettled,
                receiver_settle_mode,
                ..
            } => {
                // TODO: verfify role?
                let echo = if settled {
                    // TODO: Reply with disposition?
                    // Upon receiving the updated delivery state from the receiver, the sender will, if it has not already spontaneously
                    // attained a terminal state (e.g., through the expiry of the TTL at the sender), update its view of the state and
                    // communicate this back to the sending application.

                    // Since we are settling (ie. forgetting) this message, we don't care whether the
                    // receiving end is alive or not
                    let _result = remove_from_unsettled(unsettled, &delivery_tag)
                        .await
                        .map(|msg| msg.settle_with_state(state));
                    false
                } else {
                    let is_terminal = match &state {
                        Some(s) => s.is_terminal(),
                        None => false, // Probably should not assume the state is not specified
                    };
                    {
                        let mut guard = unsettled.write().await;
                        // Once the receiving application has finished processing the message,
                        // it indicates to the link endpoint a **terminal delivery state** that
                        // reflects the outcome of the application processing
                        if is_terminal {
                            let _result = remove_from_unsettled(unsettled, &delivery_tag)
                                .await
                                .map(|msg| msg.settle_with_state(state));
                        } else {
                            if let Some(msg) = guard.get_mut(&delivery_tag) {
                                if let Some(state) = state {
                                    *msg.state_mut() = state;
                                }
                            }
                        }
                    }
                    // If the receiver is in mode Second, it will send a non-settled terminal state
                    // to indicate end of processing
                    match receiver_settle_mode {
                        ReceiverSettleMode::First => {
                            // The receiver will spontaneously settle all incoming transfers.
                            false
                        }
                        ReceiverSettleMode::Second => {
                            // The receiver will only settle after sending the disposition to
                            // the sender and receiving a disposition indicating settlement of the
                            // delivery from the sender.

                            is_terminal
                        }
                    }
                };

                echo
            }
            LinkHandle::Receiver { unsettled, .. } => {
                if settled {
                    let _state = remove_from_unsettled(unsettled, &delivery_tag).await;
                } else {
                    let mut guard = unsettled.write().await;
                    if let Some(msg_state) = guard.get_mut(&delivery_tag) {
                        if let Some(state) = state {
                            *msg_state = state;
                        }
                    }
                }

                // Only the sender needs to auto-reply to receiver's disposition, thus
                // `echo = false`
                false
            }
        }
    }

    pub(crate) async fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<(DeliveryNumber, DeliveryTag)>, session::Error> {
        match self {
            LinkHandle::Sender { .. } => {
                // TODO: This should not happen, but should the link detach if this happens?
                todo!()
            }
            LinkHandle::Receiver {
                tx,
                receiver_settle_mode,
                more,
                ..
            } => {
                let settled = transfer.settled.unwrap_or_else(|| false);
                let delivery_id = transfer.delivery_id;
                let delivery_tag = transfer.delivery_tag.clone();
                let transfer_more = transfer.more;

                tx.send(LinkFrame::Transfer {
                    performative: transfer,
                    payload,
                })
                .await
                .map_err(|_| SessionError::UnattachedHandle)?;

                if !settled {
                    if let ReceiverSettleMode::Second = receiver_settle_mode {
                        // The delivery-id MUST be supplied on the first transfer of a
                        // multi-transfer delivery.
                        // And self.more should be false upon the first transfer
                        if *more == false {
                            // The same delivery ID should be used for a multi-transfer delivery
                            match (delivery_id, delivery_tag) {
                                (Some(id), Some(tag)) => return Ok(Some((id, tag))),
                                _ => {
                                    // This should be an error, but it will be handled by
                                    // the link instead of the session. So just return a None
                                    return Ok(None);
                                }
                            }
                        }
                        // The last transfer of multi-transfer delivery should have
                        // `more` set to false
                        *more = transfer_more;
                    }
                }
                Ok(None)
            }
        }
    }

    // pub(crate) async fn on_outgoing_disposition(
    //     &mut self,
    //     delivery_tag: &DeliveryTag,
    //     settled: bool,
    //     state: Option<DeliveryState>,
    // ) -> Result<Disposition, Error> {
    //     match self {
    //         LinkHandle::Sender { unsettled, .. } => {
    //             todo!()
    //         },
    //         LinkHandle::Receiver { unsettled, .. } => {

    //         }
    //     }

    //     todo!()
    // }
}

pub(crate) async fn remove_from_unsettled<M>(
    unsettled: &RwLock<UnsettledMap<M>>,
    key: &DeliveryTag,
) -> Option<M> {
    let mut lock = unsettled.write().await;
    lock.remove(key)
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
    match remote_attach.target.is_some() || remote_attach.source.is_some() {
        true => {
            if let Err(e) = link.on_incoming_attach(remote_attach).await {
                // Should any error happen handling remote
                panic!("{:?}", e);
            }
        }
        false => {
            // If no target or source is supplied with the remote attach frame,
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
    use crate::link::state::LinkFlowStateInner;

    #[tokio::test]
    async fn test_producer_notify() {
        use std::sync::Arc;
        use tokio::sync::Notify;

        use super::*;
        use crate::util::{Produce, Producer};

        let notifier = Arc::new(Notify::new());
        let state = LinkFlowState::sender(LinkFlowStateInner {
            initial_delivery_count: 0,
            delivery_count: 0,
            link_credit: 0,
            available: 0,
            drain: false,
            properties: None,
        });
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
