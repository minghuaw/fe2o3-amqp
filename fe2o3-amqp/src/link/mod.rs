//! Implements AMQP1.0 Link

mod frame;
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use bytes::Buf;
use fe2o3_amqp_types::{
    definitions::{
        self, AmqpError, DeliveryNumber, DeliveryTag, Handle, MessageFormat, ReceiverSettleMode,
        Role, SenderSettleMode, SequenceNo, SessionError,
    },
    messaging::{Accepted, DeliveryState, Message, Received, Source, Target, TargetArchetype},
    performatives::{Attach, Detach, Disposition, Transfer},
    primitives::Symbol,
};
pub(crate) use frame::*;
pub mod builder;
pub mod delivery;
mod error;
pub mod receiver;
mod receiver_link;
pub mod sender;
mod sender_link;

pub(crate) mod state;

pub(crate) mod target_archetype;

pub use error::*;

pub use receiver::Receiver;
pub use sender::Sender;
use serde_amqp::from_reader;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, instrument, trace};

use crate::{
    endpoint::{self, InputHandle, LinkFlow, OutputHandle, Settlement},
    link::{self, delivery::UnsettledMessage},
    util::{Consumer, Producer},
    Payload,
};

use self::{
    delivery::Delivery,
    state::{LinkFlowState, LinkState, UnsettledMap},
    target_archetype::VerifyTargetArchetype,
};

/// Default amount of link credit
pub const DEFAULT_CREDIT: SequenceNo = 200;

pub(crate) type SenderFlowState = Consumer<Arc<LinkFlowState<role::Sender>>>;
pub(crate) type ReceiverFlowState = Arc<LinkFlowState<role::Receiver>>;

/// Type alias for sender link that ONLY represents the inner state of a Sender
pub(crate) type SenderLink<T> = Link<role::Sender, T, SenderFlowState, UnsettledMessage>;

/// Type alias for receiver link that ONLY represents the inner state of receiver
pub(crate) type ReceiverLink<T> = Link<role::Receiver, T, ReceiverFlowState, DeliveryState>;

pub(crate) type ArcSenderUnsettledMap = Arc<RwLock<UnsettledMap<UnsettledMessage>>>;
pub(crate) type ArcReceiverUnsettledMap = Arc<RwLock<UnsettledMap<DeliveryState>>>;

// const CLOSED: u8 = 0b0000_0100;
// const DETACHED: u8 = 0b0000_0010;

pub mod role {
    //! Type state definition of link role

    use fe2o3_amqp_types::definitions::Role;

    /// Type state for link::builder::Builder
    #[derive(Debug)]
    pub struct Sender {}

    /// Type state for link::builder::Builder
    #[derive(Debug)]
    pub struct Receiver {}

    // /// Type state for link::builder::Builder
    // #[cfg(feature = "transaction")]
    // #[derive(Debug)]
    // pub struct Controller {}

    pub(crate) trait IntoRole {
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

    // #[cfg(feature = "transaction")]
    // impl IntoRole for Controller {
    //     fn into_role() -> Role {
    //         Role::Sender
    //     }
    // }
}

/// Manages the link state
///
/// # Type Parameters
///
/// R: role
///
/// T: target
///
/// F: link flow state
///
/// M: unsettledMessage type
#[derive(Debug)]
pub struct Link<R, T, F, M> {
    pub(crate) role: PhantomData<R>,

    pub(crate) local_state: LinkState,
    // pub(crate) state_code: Arc<AtomicU8>,
    pub(crate) name: String,

    pub(crate) output_handle: Option<OutputHandle>, // local handle
    pub(crate) input_handle: Option<InputHandle>,   // remote handle

    /// The `Sender` will manage whether to wait for incoming disposition
    pub(crate) snd_settle_mode: SenderSettleMode,
    pub(crate) rcv_settle_mode: ReceiverSettleMode,

    pub(crate) source: Option<Source>, // TODO: Option?
    pub(crate) target: Option<T>,      // TODO: Option?

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

impl<R, T, F, M> Link<R, T, F, M> {
    pub(crate) fn error_if_closed(&self) -> Result<(), definitions::Error>
    where
        R: role::IntoRole + Send + Sync,
        F: AsRef<LinkFlowState<R>> + Send + Sync,
        M: AsRef<DeliveryState> + AsMut<DeliveryState> + Send + Sync,
    {
        match self.local_state {
            LinkState::Unattached
            | LinkState::AttachSent
            | LinkState::AttachReceived
            | LinkState::Attached
            | LinkState::DetachSent
            | LinkState::DetachReceived
            | LinkState::Detached
            | LinkState::CloseSent
            | LinkState::CloseReceived => Ok(()),
            LinkState::Closed => Err(definitions::Error::new(
                AmqpError::NotAllowed,
                "Link is permanently closed".to_string(),
                None,
            )),
        }
    }
}

impl<R, T, F, M> Link<R, T, F, M>
where
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
    F: AsRef<LinkFlowState<R>> + Send + Sync,
    M: AsRef<DeliveryState> + AsMut<DeliveryState> + Send + Sync,
{
    pub(crate) async fn on_incoming_attach_inner(
        &mut self,
        handle: Handle,
        target: Option<Box<TargetArchetype>>,
        role: Role,
        source: Option<Box<Source>>,
        snd_settle_mode: SenderSettleMode,
        initial_delivery_count: Option<u32>,
        rcv_settle_mode: ReceiverSettleMode,
        max_message_size: Option<u64>,
    ) -> Result<(), AttachError> {
        match self.local_state {
            LinkState::AttachSent => self.local_state = LinkState::Attached,
            LinkState::Unattached => self.local_state = LinkState::AttachReceived,
            LinkState::Detached => self.local_state = LinkState::AttachReceived, // re-attaching
            _ => return Err(AttachError::illegal_state(None)),
        };

        self.input_handle = Some(InputHandle::from(handle));

        let target = target.ok_or(AttachError::TargetIsNone).map(|t| {
            T::try_from(*t).map_err(|_| AttachError::not_implemented("Target mismatch".to_string()))
        })??;

        // When resuming a link, it is possible that the properties of the source and target have changed while the link
        // was suspended. When this happens, the termini properties communicated in the source and target fields of the
        // attach frames could be in conflict.
        match role {
            // Remote attach is from sender, local is receiver
            Role::Sender => {
                self.on_incoming_attach_as_receiver(
                    source,
                    snd_settle_mode,
                    initial_delivery_count,
                    target,
                )
                .await?;
            }
            // Remote attach is from receiver, local is sender
            Role::Receiver => {
                self.on_incoming_attach_as_sender(target, rcv_settle_mode)
                    .await?;
            }
        }

        // set max message size
        // If this field is zero or unset, there is no maximum size imposed by the link endpoint.
        self.max_message_size = get_max_message_size(self.max_message_size, max_message_size);

        // TODO: what to do with the unattached

        Ok(())
    }

    // Handles incoming attach when the remote is sender
    pub(crate) async fn on_incoming_attach_as_receiver(
        &mut self,
        source: Option<Box<Source>>,
        snd_settle_mode: SenderSettleMode,
        initial_delivery_count: Option<u32>,
        target: T,
    ) -> Result<(), AttachError> {
        // In this case, the sender is considered to hold the authoritative version of the
        // version of the source properties

        let source = source.ok_or(AttachError::SourceIsNone)?;
        self.source = Some(*source);

        // The receiver SHOULD respect the sender’s desired settlement mode if the sender
        // initiates the attach exchange and the receiver supports the desired mode.
        self.snd_settle_mode = snd_settle_mode;

        // The delivery-count is initialized by the sender when a link endpoint is
        // created, and is incremented whenever a message is sent
        let initial_delivery_count = match initial_delivery_count {
            Some(val) => val,
            None => {
                return Err(AttachError::not_allowed(
                    "This MUST NOT be null if role is sender".to_string(),
                ))
            }
        };

        // TODO: **the receiver is considered to hold the authoritative version of the target properties**,
        // Is this verification necessary?
        if let Some(local) = &self.target {
            local
                .verify_as_receiver(&target)
                .map_err(|err| (AttachError::Local(err)))?;
        }

        self.flow_state
            .as_ref()
            .initial_delivery_count_mut(|_| initial_delivery_count)
            .await;
        self.flow_state
            .as_ref()
            .delivery_count_mut(|_| initial_delivery_count)
            .await;

        Ok(())
    }

    pub(crate) async fn on_incoming_attach_as_sender(
        &mut self,
        target: T,
        rcv_settle_mode: ReceiverSettleMode,
    ) -> Result<(), AttachError> {
        // Note that it is the responsibility of the transaction controller to
        // verify that the capabilities of the controller meet its requirements.
        if let Some(local) = &self.target {
            local
                .verify_as_sender(&target)
                .map_err(|err| (AttachError::Local(err)))?;
        }
        self.target = Some(target);

        // The sender SHOULD respect the receiver’s desired settlement mode if the receiver
        // initiates the attach exchange and the sender supports the desired mode
        self.rcv_settle_mode = rcv_settle_mode;

        Ok(())
    }
}

impl<R, T, F, M> endpoint::Link for Link<R, T, F, M>
where
    R: role::IntoRole + Send + Sync,
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
    F: AsRef<LinkFlowState<R>> + Send + Sync,
    M: AsRef<DeliveryState> + AsMut<DeliveryState> + Send + Sync,
{
    fn role() -> Role {
        R::into_role()
    }
}

impl<R, T, F, M> endpoint::LinkExt for Link<R, T, F, M>
where
    R: role::IntoRole + Send + Sync,
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
    F: AsRef<LinkFlowState<R>> + Send + Sync,
    M: AsRef<DeliveryState> + AsMut<DeliveryState> + Send + Sync,
{
    type FlowState = F;
    type Unsettled = Arc<RwLock<UnsettledMap<M>>>;
    type Target = T;

    fn name(&self) -> &str {
        &self.name
    }

    fn output_handle(&self) -> &Option<OutputHandle> {
        &self.output_handle
    }

    fn output_handle_mut(&mut self) -> &mut Option<OutputHandle> {
        &mut self.output_handle
    }

    fn flow_state(&self) -> &Self::FlowState {
        &self.flow_state
    }

    fn unsettled(&self) -> &Self::Unsettled {
        &self.unsettled
    }

    fn rcv_settle_mode(&self) -> &ReceiverSettleMode {
        &self.rcv_settle_mode
    }

    fn target(&self) -> &Option<Self::Target> {
        &self.target
    }
}

#[async_trait]
impl<R, T, F, M> endpoint::LinkAttach for Link<R, T, F, M>
where
    R: role::IntoRole + Send + Sync,
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
    F: AsRef<LinkFlowState<R>> + Send + Sync,
    M: AsRef<DeliveryState> + AsMut<DeliveryState> + Send + Sync,
{
    type AttachError = link::AttachError;

    async fn on_incoming_attach(&mut self, remote_attach: Attach) -> Result<(), Self::AttachError> {
        self.on_incoming_attach_inner(
            remote_attach.handle,
            remote_attach.target,
            remote_attach.role,
            remote_attach.source,
            remote_attach.snd_settle_mode,
            remote_attach.initial_delivery_count,
            remote_attach.rcv_settle_mode,
            remote_attach.max_message_size,
        )
        .await
    }

    async fn send_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::AttachError> {
        self.error_if_closed().map_err(AttachError::Local)?;

        // Create Attach frame
        let handle = match &self.output_handle {
            Some(h) => h.clone(),
            None => {
                return Err(AttachError::Local(definitions::Error::new(
                    AmqpError::InvalidField,
                    Some("Output handle is None".into()),
                    None,
                )))
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
            val => Some(val as u64),
        };
        let initial_delivery_count = Some(self.flow_state.as_ref().initial_delivery_count().await);
        let properties = self.flow_state.as_ref().properties().await;

        let attach = Attach {
            name: self.name.clone(),
            handle: handle.into(),
            role: R::into_role(),
            snd_settle_mode: self.snd_settle_mode.clone(),
            rcv_settle_mode: self.rcv_settle_mode.clone(),
            source: self.source.clone().map(Box::new),
            target: self.target.clone().map(Into::into).map(Box::new),
            unsettled,
            incomplete_unsettled: false, // TODO: try send once and then retry if frame size too large?

            /// This MUST NOT be null if role is sender,
            /// and it is ignored if the role is receiver.
            /// See subsection 2.6.7.
            initial_delivery_count,

            max_message_size,
            offered_capabilities: self.offered_capabilities.clone().map(Into::into),
            desired_capabilities: self.desired_capabilities.clone().map(Into::into),
            properties,
        };
        let frame = LinkFrame::Attach(attach);

        match self.local_state {
            LinkState::Unattached
            | LinkState::Detached // May attempt to re-attach
            | LinkState::DetachSent => {
                writer.send(frame).await
                    .map_err(|_| AttachError::IllegalSessionState)?;
                self.local_state = LinkState::AttachSent
            }
            LinkState::AttachReceived => {
                writer.send(frame).await
                    .map_err(|_| AttachError::IllegalSessionState)?;
                // self.state_code.fetch_and(0b0000_0000, Ordering::Release);
                self.local_state = LinkState::Attached
            }
            _ => return Err(AttachError::Local(definitions::Error::new(
                AmqpError::IllegalState,
                None,
                None
            ))),
        }

        Ok(())
    }
}

#[async_trait]
impl<R, T, F, M> endpoint::LinkDetach for Link<R, T, F, M>
where
    R: role::IntoRole + Send + Sync,
    T: Send,
    F: AsRef<LinkFlowState<R>> + Send + Sync,
    M: AsRef<DeliveryState> + AsMut<DeliveryState> + Send + Sync,
{
    type DetachError = definitions::Error;

    /// Closing or not isn't taken care of here but outside
    #[instrument(skip_all)]
    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::DetachError> {
        trace!(detach = ?detach);

        match detach.closed {
            true => match self.local_state {
                LinkState::Attached
                | LinkState::AttachSent
                | LinkState::AttachReceived
                | LinkState::DetachSent
                | LinkState::DetachReceived => self.local_state = LinkState::CloseReceived,
                LinkState::CloseSent => {
                    self.local_state = LinkState::Closed;
                    let _ = self.output_handle.take();
                }
                _ => {
                    return Err(definitions::Error::new(
                        AmqpError::IllegalState,
                        Some("Illegal local state".into()),
                        None,
                    ))
                }
            },
            false => {
                match self.local_state {
                    LinkState::Attached => self.local_state = LinkState::DetachReceived,
                    LinkState::DetachSent => {
                        self.local_state = LinkState::Detached;
                        // Dropping output handle as it is already detached
                        let _ = self.output_handle.take();
                    }
                    _ => {
                        return Err(definitions::Error::new(
                            AmqpError::IllegalState,
                            Some("Illegal local state".into()),
                            None,
                        ))
                    }
                }
            }
        }

        if let Some(err) = detach.error {
            return Err(err);
        }
        Ok(())
    }

    #[instrument(skip_all)]
    async fn send_detach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        closed: bool,
        error: Option<Self::DetachError>,
    ) -> Result<(), Self::DetachError> {
        match self.local_state {
            LinkState::Attached => {
                self.local_state = LinkState::DetachSent;
            }
            LinkState::DetachReceived => {
                self.local_state = LinkState::Detached;
            }
            _ => return Err(AmqpError::IllegalState.into()),
        }

        match self.output_handle.take() {
            Some(handle) => {
                let detach = Detach {
                    handle: handle.into(),
                    closed,
                    error,
                };

                debug!("Sending detach: {:?}", detach);

                writer.send(LinkFrame::Detach(detach)).await.map_err(|_| {
                    definitions::Error::new(
                        AmqpError::IllegalState,
                        Some("Session must have dropped".to_string()),
                        None,
                    )
                })?;
            }
            None => return Err(definitions::Error::new(AmqpError::IllegalState, None, None)),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum LinkRelay<O> {
    Sender {
        tx: mpsc::Sender<LinkIncomingItem>,
        output_handle: O,
        // This should be wrapped inside a Producer because the SenderLink
        // needs to consume link credit from LinkFlowState
        flow_state: Producer<Arc<LinkFlowState<role::Sender>>>,
        unsettled: Arc<RwLock<UnsettledMap<UnsettledMessage>>>,
        receiver_settle_mode: ReceiverSettleMode,
        // state_code: Arc<AtomicU8>,
    },
    Receiver {
        tx: mpsc::Sender<LinkIncomingItem>,
        output_handle: O,
        flow_state: ReceiverFlowState,
        unsettled: Arc<RwLock<UnsettledMap<DeliveryState>>>,
        receiver_settle_mode: ReceiverSettleMode,
        // state_code: Arc<AtomicU8>,
        more: bool,
    },
}

impl LinkRelay<()> {
    pub fn with_output_handle(self, output_handle: OutputHandle) -> LinkRelay<OutputHandle> {
        match self {
            LinkRelay::Sender {
                tx,
                flow_state,
                unsettled,
                receiver_settle_mode,
                ..
            } => LinkRelay::Sender {
                tx,
                output_handle,
                flow_state,
                unsettled,
                receiver_settle_mode,
            },
            LinkRelay::Receiver {
                tx,
                flow_state,
                unsettled,
                receiver_settle_mode,
                more,
                ..
            } => LinkRelay::Receiver {
                tx,
                output_handle,
                flow_state,
                unsettled,
                receiver_settle_mode,
                more,
            },
        }
    }
}

impl LinkRelay<OutputHandle> {
    pub(crate) fn output_handle(&self) -> &OutputHandle {
        match self {
            Self::Sender { output_handle, .. } => output_handle,
            Self::Receiver { output_handle, .. } => output_handle,
        }
    }

    pub(crate) async fn send(
        &mut self,
        frame: LinkFrame,
    ) -> Result<(), mpsc::error::SendError<LinkFrame>> {
        match self {
            LinkRelay::Sender { tx, .. } => tx.send(frame).await,
            LinkRelay::Receiver { tx, .. } => tx.send(frame).await,
        }
    }

    pub(crate) async fn on_incoming_flow(&mut self, flow: LinkFlow) -> Option<LinkFlow> {
        match self {
            LinkRelay::Sender {
                flow_state,
                output_handle,
                ..
            } => {
                flow_state
                    .on_incoming_flow(flow, output_handle.clone())
                    .await
            }
            LinkRelay::Receiver {
                flow_state,
                output_handle,
                ..
            } => {
                flow_state
                    .on_incoming_flow(flow, output_handle.clone())
                    .await
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
            LinkRelay::Sender {
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
                        } else if let Some(msg) = guard.get_mut(&delivery_tag) {
                            if let Some(state) = state {
                                *msg.state_mut() = state;
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
            LinkRelay::Receiver { unsettled, .. } => {
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

    /// LinkRelay operates in session's event loop
    pub(crate) async fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<(DeliveryNumber, DeliveryTag)>, (bool, definitions::Error)> {
        match self {
            LinkRelay::Sender { .. } => {
                // TODO: This should not happen, but should the link detach if this happens?
                Err((
                    true, // Closing the link
                    definitions::Error::new(
                        AmqpError::NotAllowed,
                        Some("Sender should never receive a transfer".to_string()),
                        None,
                    ),
                ))
            }
            LinkRelay::Receiver {
                tx,
                receiver_settle_mode,
                more,
                ..
            } => {
                let settled = transfer.settled.unwrap_or(false);
                let delivery_id = transfer.delivery_id;
                let delivery_tag = transfer.delivery_tag.clone();
                let transfer_more = transfer.more;

                tx.send(LinkFrame::Transfer {
                    input_handle: InputHandle::from(transfer.handle.clone()),
                    performative: transfer,
                    payload,
                })
                .await
                .map_err(|_| {
                    (
                        true,
                        definitions::Error::new(SessionError::UnattachedHandle, None, None),
                    )
                })?;

                if !settled {
                    if let ReceiverSettleMode::Second = receiver_settle_mode {
                        // The delivery-id MUST be supplied on the first transfer of a
                        // multi-transfer delivery.
                        // And self.more should be false upon the first transfer
                        if !(*more) {
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

    pub async fn on_incoming_detach(
        &mut self,
        detach: Detach,
    ) -> Result<(), mpsc::error::SendError<LinkFrame>> {
        match self {
            LinkRelay::Sender { tx, .. } => {
                // state_code.fetch_or(DETACHED, Ordering::Release);
                // if detach.closed {
                //     state_code.fetch_or(CLOSED, Ordering::Release);
                // }
                tx.send(LinkFrame::Detach(detach)).await?;
            }
            LinkRelay::Receiver { tx, .. } => {
                // state_code.fetch_or(DETACHED, Ordering::Release);
                // if detach.closed {
                //     state_code.fetch_or(CLOSED, Ordering::Release);
                // }
                tx.send(LinkFrame::Detach(detach)).await?;
            }
        }
        Ok(())
    }
}

pub(crate) async fn remove_from_unsettled<M>(
    unsettled: &RwLock<UnsettledMap<M>>,
    key: &DeliveryTag,
) -> Option<M> {
    let mut lock = unsettled.write().await;
    lock.remove(key)
}

pub(crate) async fn do_attach<L>(
    link: &mut L,
    writer: &mpsc::Sender<LinkFrame>,
    reader: &mut mpsc::Receiver<LinkFrame>,
) -> Result<(), AttachError>
where
    L: endpoint::LinkAttach<AttachError = AttachError>
        + endpoint::LinkDetach<DetachError = definitions::Error>
        + endpoint::Link,
{
    // Send an Attach frame
    endpoint::LinkAttach::send_attach(link, writer).await?;

    // Wait for an Attach frame
    let frame = reader
        .recv()
        .await
        .ok_or_else(|| AttachError::illegal_state(None))?;

    let remote_attach = match frame {
        LinkFrame::Attach(attach) => attach,
        // LinkFrame::Detach(detach) => {
        //     return Err(Error::Detached(DetachError {
        //         link: None,
        //         is_closed_by_remote: detach.closed,
        //         error: detach.error,
        //     }))
        // }
        // TODO: how to handle this?
        _ => return Err(AttachError::illegal_state("expecting Attach".to_string())),
    };

    // Note that if the application chooses not to create a terminus,
    // the session endpoint will still create a link endpoint and issue
    // an attach indicating that the link endpoint has no associated
    // local terminus. In this case, the session endpoint MUST immediately
    // detach the newly created link endpoint.
    match remote_attach.target.is_none() || remote_attach.source.is_none() {
        true => {
            // If no target or source is supplied with the remote attach frame,
            // an immediate detach should be expected
            tracing::error!("Found null source or target");
            expect_detach_then_detach(link, writer, reader)
                .await
                .map_err(|_| AttachError::illegal_state("Expecting detach".to_string()))?;
        }
        false => {
            if let Err(e) = link.on_incoming_attach(remote_attach).await {
                // Should any error happen handling remote
                tracing::error!("{:?}", e);
            }
        }
    }

    Ok(())
}

pub(crate) async fn expect_detach_then_detach<L>(
    link: &mut L,
    writer: &mpsc::Sender<LinkFrame>,
    reader: &mut mpsc::Receiver<LinkFrame>,
) -> Result<(), Error>
where
    L: endpoint::LinkAttach<AttachError = AttachError>
        + endpoint::LinkDetach<DetachError = definitions::Error>,
{
    let frame = reader
        .recv()
        .await
        .ok_or_else(|| Error::expecting_frame("Detach"))?;

    let _remote_detach = match frame {
        LinkFrame::Detach(detach) => detach,
        _ => return Err(Error::expecting_frame("Detach")),
    };

    link.send_detach(writer, false, None)
        .await
        .map_err(Error::Local)?;
    Ok(())
}

pub(crate) fn get_max_message_size(local: u64, remote: Option<u64>) -> u64 {
    let remote_max_msg_size = remote.unwrap_or(0);
    match local {
        0 => remote_max_msg_size,
        val => {
            if remote_max_msg_size == 0 {
                val
            } else {
                u64::min(val, remote_max_msg_size)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::link::state::LinkFlowStateInner;

    #[tokio::test]
    async fn test_producer_notify() {
        use std::sync::Arc;
        use tokio::sync::Notify;

        use super::*;
        use crate::endpoint::OutputHandle;
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
            let item = (LinkFlow::default(), OutputHandle(0));
            producer.produce(item).await;
        });

        notified.await;
        println!("wait passed");

        handle.await.unwrap();
    }
}
