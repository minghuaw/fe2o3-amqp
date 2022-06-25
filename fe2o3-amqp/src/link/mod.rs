//! Implements AMQP1.0 Link

mod frame;
use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use bytes::Buf;
use fe2o3_amqp_types::{
    definitions::{
        self, AmqpError, DeliveryNumber, DeliveryTag, MessageFormat, ReceiverSettleMode, Role,
        SenderSettleMode, SequenceNo, SessionError,
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

pub(crate) mod shared_inner;

pub use error::*;

pub use receiver::Receiver;
pub use sender::Sender;
use serde_amqp::from_reader;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, instrument, trace};

use crate::{
    control::SessionControl,
    endpoint::{self, InputHandle, LinkAttach, LinkDetach, LinkFlow, OutputHandle, Settlement},
    link::delivery::UnsettledMessage,
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

    /// Marker trait for role
    pub trait IntoRole {
        /// Get the role
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

impl<R, T, F, M> Link<R, T, F, M>
where
    R: role::IntoRole + Send + Sync,
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
    F: AsRef<LinkFlowState<R>> + Send + Sync,
    M: AsRef<DeliveryState> + AsMut<DeliveryState> + Send + Sync,
{
    pub(crate) async fn send_attach_inner(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
    ) -> Result<(), SendAttachErrorKind> {
        // self.error_if_closed().map_err(|_| SendAttachErrorKind::LinkClosed)?; // This should be unreachable

        // Create Attach frame
        let handle = match &self.output_handle {
            Some(h) => h.clone(),
            None => return Err(SendAttachErrorKind::IllegalState),
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
                    .map_err(|_| SendAttachErrorKind::IllegalSessionState)?;
                self.local_state = LinkState::AttachSent
            }
            LinkState::AttachReceived => {
                writer.send(frame).await
                    .map_err(|_| SendAttachErrorKind::IllegalSessionState)?;
                // self.state_code.fetch_and(0b0000_0000, Ordering::Release);
                self.local_state = LinkState::Attached
            }
            _ => return Err(SendAttachErrorKind::IllegalState),
        }

        Ok(())
    }
}

impl<T> endpoint::Link for SenderLink<T>
where
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
{
    fn role() -> Role {
        Role::Sender
    }
}

#[async_trait]
impl<T> endpoint::LinkExt for SenderLink<T>
where
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
{
    type FlowState = SenderFlowState;
    type Unsettled = Arc<RwLock<UnsettledMap<UnsettledMessage>>>;
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

    async fn negotiate_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
    ) -> Result<(), SenderAttachError> {
        // Send out local attach
        self.send_attach(writer).await?;

        // Wait for remote attach
        let remote_attach = match reader
            .recv()
            .await
            .ok_or(SenderAttachError::IllegalSessionState)?
        {
            LinkFrame::Attach(attach) => attach,
            _ => return Err(SenderAttachError::NonAttachFrameReceived),
        };

        self.on_incoming_attach(remote_attach).await
    }

    async fn handle_attach_error(
        &mut self,
        attach_error: SenderAttachError,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
        session: &mpsc::Sender<SessionControl>,
    ) -> SenderAttachError {
        match attach_error {
            SenderAttachError::IllegalSessionState
            | SenderAttachError::IllegalState
            | SenderAttachError::NonAttachFrameReceived
            | SenderAttachError::ExpectImmediateDetach => attach_error,

            SenderAttachError::DuplicatedLinkName => {
                let error = definitions::Error::new(
                    SessionError::HandleInUse,
                    "Link name is in use".to_string(),
                    None,
                );
                session
                    .send(SessionControl::End(Some(error)))
                    .await
                    .map(|_| attach_error)
                    .unwrap_or(SenderAttachError::IllegalSessionState)
            }

            SenderAttachError::IncomingSourceIsNone | SenderAttachError::IncomingTargetIsNone => {
                // Just send detach immediately
                let err = self.send_detach(writer, true, None)
                    .await
                    .map(|_| attach_error)
                    .unwrap_or(SenderAttachError::IllegalSessionState);
                match reader.recv().await {
                    Some(LinkFrame::Detach(remote_detach)) => {
                        let _ = self.on_incoming_detach(remote_detach).await; // FIXME: hadnle detach errors?
                        err
                    }
                    Some(_) => SenderAttachError::NonAttachFrameReceived,
                    None => SenderAttachError::IllegalSessionState,
                }
            }

            SenderAttachError::CoordinatorIsNotImplemented
            | SenderAttachError::AddressIsNoneWhenDynamicIsTrue
            | SenderAttachError::DesireTxnCapabilitiesNotSupported
            | SenderAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse => {
                match (&attach_error).try_into() {
                    Ok(error) => {
                        match self.send_detach(writer, true, Some(error)).await {
                            Ok(_) => match reader.recv().await {
                                Some(LinkFrame::Detach(remote_detach)) => {
                                    let _ = self.on_incoming_detach(remote_detach).await; // FIXME: hadnle detach errors?
                                    attach_error
                                }
                                Some(_) => SenderAttachError::NonAttachFrameReceived,
                                None => SenderAttachError::IllegalSessionState,
                            },
                            Err(_) => SenderAttachError::IllegalSessionState,
                        }
                    }
                    Err(_) => attach_error,
                }
            }
        }
    }
}

impl<T> endpoint::Link for ReceiverLink<T>
where
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
{
    fn role() -> Role {
        Role::Receiver
    }
}

#[async_trait]
impl<T> endpoint::LinkExt for ReceiverLink<T>
where
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
{
    type FlowState = ReceiverFlowState;
    type Unsettled = Arc<RwLock<UnsettledMap<DeliveryState>>>;
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

    async fn negotiate_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
    ) -> Result<(), ReceiverAttachError> {
        // Send out local attach
        self.send_attach(writer).await?;

        // Wait for remote attach
        let remote_attach = match reader
            .recv()
            .await
            .ok_or(ReceiverAttachError::IllegalSessionState)?
        {
            LinkFrame::Attach(attach) => attach,
            _ => return Err(ReceiverAttachError::NonAttachFrameReceived),
        };

        self.on_incoming_attach(remote_attach).await
    }

    async fn handle_attach_error(
        &mut self,
        attach_error: ReceiverAttachError,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
        session: &mpsc::Sender<SessionControl>,
    ) -> ReceiverAttachError {
        match attach_error {
            ReceiverAttachError::IllegalSessionState
            | ReceiverAttachError::IllegalState
            | ReceiverAttachError::NonAttachFrameReceived
            | ReceiverAttachError::ExpectImmediateDetach => attach_error,

            ReceiverAttachError::DuplicatedLinkName => {
                let error = definitions::Error::new(
                    SessionError::HandleInUse,
                    "Link name is in use".to_string(),
                    None,
                );
                session
                    .send(SessionControl::End(Some(error)))
                    .await
                    .map(|_| attach_error)
                    .unwrap_or(ReceiverAttachError::IllegalSessionState)
            }
            ReceiverAttachError::IncomingSourceIsNone
            | ReceiverAttachError::IncomingTargetIsNone => match reader.recv().await {
                Some(LinkFrame::Detach(remote_detach)) => self
                    .send_detach(writer, remote_detach.closed, None)
                    .await
                    .map(|_| attach_error)
                    .unwrap_or(ReceiverAttachError::IllegalSessionState),
                Some(_) => ReceiverAttachError::NonAttachFrameReceived,
                None => ReceiverAttachError::IllegalSessionState,
            },

            ReceiverAttachError::CoordinatorIsNotImplemented
            | ReceiverAttachError::InitialDeliveryCountIsNone
            | ReceiverAttachError::AddressIsSomeWhenDynamicIsTrue
            | ReceiverAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse => {
                match (&attach_error).try_into() {
                    Ok(error) => {
                        match self.send_detach(writer, true, Some(error)).await {
                            Ok(_) => match reader.recv().await {
                                Some(LinkFrame::Detach(remote_detach)) => {
                                    let _ = self.on_incoming_detach(remote_detach).await; // FIXME: how to handle this?
                                    attach_error
                                }
                                Some(_) => ReceiverAttachError::NonAttachFrameReceived,
                                None => ReceiverAttachError::IllegalSessionState,
                            },
                            Err(_) => ReceiverAttachError::IllegalSessionState,
                        }
                    }
                    Err(_) => attach_error,
                }
            }
        }
    }
}

#[async_trait]
impl<T> endpoint::LinkAttach for SenderLink<T>
where
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
{
    type AttachError = SenderAttachError;

    async fn on_incoming_attach(&mut self, remote_attach: Attach) -> Result<(), Self::AttachError> {
        match self.local_state {
            LinkState::AttachSent => self.local_state = LinkState::Attached,
            LinkState::Unattached => self.local_state = LinkState::AttachReceived,
            LinkState::Detached => self.local_state = LinkState::AttachReceived, // re-attaching
            _ => return Err(SenderAttachError::IllegalState),
        };

        self.input_handle = Some(InputHandle::from(remote_attach.handle));

        let target = remote_attach
            .target
            .map(|t| T::try_from(*t))
            .transpose()
            .map_err(|_| SenderAttachError::CoordinatorIsNotImplemented)?;

        // Note that it is the responsibility of the transaction controller to
        // verify that the capabilities of the controller meet its requirements.
        match (&self.target, &target) {
            (Some(local_target), Some(remote_target)) => {
                local_target.verify_as_sender(remote_target)?
            }
            (_, None) => return Err(SenderAttachError::IncomingTargetIsNone),
            _ => {}
        }
        self.target = target;

        // The sender SHOULD respect the receiver’s desired settlement mode if the receiver
        // initiates the attach exchange and the sender supports the desired mode
        self.rcv_settle_mode = remote_attach.rcv_settle_mode;

        self.max_message_size =
            get_max_message_size(self.max_message_size, remote_attach.max_message_size);

        Ok(())
    }

    async fn send_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::AttachError> {
        self.send_attach_inner(writer).await?;
        Ok(())
    }
}

#[async_trait]
impl<T> endpoint::LinkAttach for ReceiverLink<T>
where
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
{
    type AttachError = ReceiverAttachError;

    async fn on_incoming_attach(&mut self, remote_attach: Attach) -> Result<(), Self::AttachError> {
        match self.local_state {
            LinkState::AttachSent => self.local_state = LinkState::Attached,
            LinkState::Unattached => self.local_state = LinkState::AttachReceived,
            LinkState::Detached => self.local_state = LinkState::AttachReceived, // re-attaching
            _ => return Err(ReceiverAttachError::IllegalState),
        };

        self.input_handle = Some(InputHandle::from(remote_attach.handle));

        // In this case, the sender is considered to hold the authoritative version of the
        // version of the source properties

        let source = remote_attach
            .source
            .ok_or(ReceiverAttachError::IncomingSourceIsNone)?;
        self.source = Some(*source);

        // The receiver SHOULD respect the sender’s desired settlement mode if the sender
        // initiates the attach exchange and the receiver supports the desired mode.
        self.snd_settle_mode = remote_attach.snd_settle_mode;

        // The delivery-count is initialized by the sender when a link endpoint is
        // created, and is incremented whenever a message is sent
        let initial_delivery_count = remote_attach
            .initial_delivery_count
            .ok_or(ReceiverAttachError::InitialDeliveryCountIsNone)?;

        let target = remote_attach
            .target
            .map(|t| T::try_from(*t))
            .transpose()
            .map_err(|_| ReceiverAttachError::CoordinatorIsNotImplemented)?;

        // TODO: **the receiver is considered to hold the authoritative version of the target properties**,
        // Is this verification necessary?
        match (&self.target, &target) {
            (Some(local_target), Some(remote_target)) => {
                local_target.verify_as_receiver(remote_target)?
            }
            (_, None) => return Err(ReceiverAttachError::IncomingTargetIsNone),
            _ => {}
        }

        self.max_message_size =
            get_max_message_size(self.max_message_size, remote_attach.max_message_size);

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

    async fn send_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::AttachError> {
        self.send_attach_inner(writer).await?;
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
    type DetachError = DetachError;

    /// Closing or not isn't taken care of here but outside
    #[instrument(skip_all)]
    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::DetachError> {
        trace!(detach = ?detach);

        match detach.closed {
            true => match self.local_state {
                LinkState::Attached | LinkState::AttachSent | LinkState::AttachReceived => {
                    self.local_state = LinkState::CloseReceived;
                    match detach.error {
                        Some(error) => Err(DetachError::RemoteClosedWithError(error)),
                        None => Ok(()),
                    }
                }
                LinkState::DetachSent => {
                    self.local_state = LinkState::CloseReceived;
                    match detach.error {
                        Some(error) => Err(DetachError::RemoteClosedWithError(error)),
                        None => Err(DetachError::ClosedByRemote),
                    }
                }
                LinkState::CloseSent => {
                    self.local_state = LinkState::Closed;
                    let _ = self.output_handle.take();
                    match detach.error {
                        Some(error) => Err(DetachError::RemoteClosedWithError(error)),
                        None => Ok(()),
                    }
                }
                _ => Err(DetachError::IllegalState),
            },
            false => {
                match self.local_state {
                    LinkState::Attached => self.local_state = LinkState::DetachReceived,
                    LinkState::DetachSent => {
                        self.local_state = LinkState::Detached;
                        // Dropping output handle as it is already detached
                        let _ = self.output_handle.take();
                    }
                    _ => return Err(DetachError::IllegalState),
                }

                match detach.error {
                    Some(error) => Err(DetachError::RemoteDetachedWithError(error)),
                    None => Ok(()),
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn send_detach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        closed: bool,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::DetachError> {
        match self.output_handle.clone() {
            Some(handle) => {
                let detach = Detach {
                    handle: handle.into(),
                    closed,
                    error,
                };

                debug!("Sending detach: {:?}", detach);

                writer
                    .send(LinkFrame::Detach(detach))
                    .await
                    .map_err(|_| DetachError::IllegalSessionState)?;

                match (&self.local_state, closed) {
                    (LinkState::Attached, false) => self.local_state = LinkState::DetachSent,
                    (LinkState::DetachReceived, false) => self.local_state = LinkState::Detached,
                    (LinkState::CloseReceived, false) => return Err(DetachError::ClosedByRemote),
                    (LinkState::Attached, true) => self.local_state = LinkState::CloseSent,
                    (LinkState::DetachReceived, true) => return Err(DetachError::DetachedByRemote),
                    (LinkState::CloseReceived, true) => self.local_state = LinkState::Closed,
                    _ => return Err(DetachError::IllegalState),
                };
                self.output_handle.take();
            }
            None => return Err(DetachError::IllegalState),
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
                tx.send(LinkFrame::Detach(detach)).await?;
            }
            LinkRelay::Receiver { tx, .. } => {
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
