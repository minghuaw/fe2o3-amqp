//! Link state and link flow state

use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use fe2o3_amqp_types::definitions::{DeliveryTag, Fields, SequenceNo};
use tokio::sync::RwLock;

use crate::{
    endpoint::{LinkFlow, OutputHandle},
    util::{Consume, Produce, Producer, ProducerState, TryConsume},
};

use super::{role, SenderFlowState, SenderTryConsumeError};

/// Link state.
///
/// There is no official definition of the link state in the specification
#[derive(Debug)]
pub enum LinkState {
    /// The initial state after initialization
    Unattached,

    /// An attach frame has been sent
    AttachSent,

    /// An attach frame has been received
    AttachReceived,

    /// The link is attached
    Attached,

    /// A non-closing detach frame has been sent
    DetachSent,

    /// A non-closing detach frame has been received
    DetachReceived,

    /// The link is detached
    Detached,

    /// A closing detach frame has been sent
    CloseSent,

    /// A closing detach has arrived
    CloseReceived,

    /// The link is closed
    Closed,
}

#[derive(Debug)]
pub(crate) struct LinkFlowStateInner {
    pub initial_delivery_count: SequenceNo,
    pub delivery_count: SequenceNo, // SequenceNo = u32
    pub link_credit: u32,
    pub available: u32,
    pub drain: bool,
    pub properties: Option<Fields>,
}

impl LinkFlowStateInner {
    pub fn as_link_flow(&self, output_handle: OutputHandle, echo: bool) -> LinkFlow {
        LinkFlow {
            handle: output_handle.into(),
            delivery_count: Some(self.delivery_count),
            link_credit: Some(self.link_credit),
            available: Some(self.available),
            drain: self.drain,
            echo,
            properties: self.properties.clone(),
        }
    }
}

/// The Sender and Receiver handle link flow control differently
#[derive(Debug)]
pub struct LinkFlowState<R> {
    // Sender(RwLock<LinkFlowStateInner>),
    // Receiver(RwLock<LinkFlowStateInner>),
    pub(crate) lock: RwLock<LinkFlowStateInner>,
    role: PhantomData<R>,
}

impl<R> LinkFlowState<R> {
    pub(crate) fn new(inner: LinkFlowStateInner) -> Self {
        Self {
            lock: RwLock::new(inner),
            role: PhantomData,
        }
    }
}

impl LinkFlowState<role::Sender> {
    pub(crate) fn sender(inner: LinkFlowStateInner) -> Self {
        Self::new(inner)
    }
}

impl LinkFlowState<role::Receiver> {
    pub(crate) fn receiver(inner: LinkFlowStateInner) -> Self {
        Self::new(inner)
    }
}

impl LinkFlowState<role::Sender> {
    /// Handles incoming Flow frame
    ///
    /// If an echo (reply with the local flow state) is requested, return an `Ok(Some(Flow))`,
    /// otherwise, return a `Ok(None)`
    #[inline]
    pub(crate) async fn on_incoming_flow(
        &self,
        flow: LinkFlow,
        output_handle: OutputHandle,
    ) -> Option<LinkFlow> {
        let mut state = self.lock.write().await;

        // delivery count
        //
        // ...
        // Only the sender MAY independently modify this field.

        // link credit
        //
        // ...
        // This means that the sender’s link-credit variable
        // MUST be set according to this formula when flow information is given by the
        // receiver:
        // link-credit_snd := delivery-count_rcv + link-credit_rcv - delivery-count_snd.
        let delivery_count_rcv = flow.delivery_count.unwrap_or(
            // In the event that the receiver does not yet know the delivery-count,
            // i.e., delivery-count_rcv is unspecified, the sender MUST assume that
            // the delivery-count_rcv is the first delivery-count_snd sent from sender
            // to receiver, i.e., the delivery-count_snd specified in the flow state
            // carried by the initial attach frame from the sender to the receiver.
            state.initial_delivery_count,
        );

        if let Some(link_credit_rcv) = flow.link_credit {
            let link_credit = delivery_count_rcv + link_credit_rcv - state.delivery_count;
            state.link_credit = link_credit;
        }

        // available
        //
        // The available variable is controlled by the sender, and indicates to the receiver,
        // that the sender could make use of the indicated amount of link-credit. Only the
        // sender can indepen- dently modify this field.

        // drain, TODO: advance the link-credit as much as possible
        //
        // The drain flag indicates how the sender SHOULD behave when insufficient messages
        // are available to consume the current link-credit. If set, the sender will (after
        // sending all available messages) advance the delivery-count as much as possible,
        // consuming all link-credit, and send the flow state to the receiver. Only the
        // receiver can independently modify this field. The sender’s value is always the
        // last known value indicated by the receiver.
        state.drain = flow.drain;
        if flow.drain {
            state.delivery_count += state.link_credit;
            state.link_credit = 0;

            return Some(state.as_link_flow(output_handle, false));
        }

        match flow.echo {
            // Should avoid constant ping-pong
            true => Some(state.as_link_flow(output_handle, false)),
            false => None,
        }
    }
}

impl LinkFlowState<role::Receiver> {
    #[inline]
    pub(crate) async fn on_incoming_flow(
        &self,
        flow: LinkFlow,
        output_handle: OutputHandle,
    ) -> Option<LinkFlow> {
        let mut state = self.lock.write().await;

        // delivery count
        //
        // The receiver’s value is calculated based on the last known
        // value from the sender and any subsequent messages received on the link. Note that,
        // despite its name, the delivery-count is not a count but a sequence number
        // initialized at an arbitrary point by the sender.
        if let Some(delivery_count) = flow.delivery_count {
            state.delivery_count = delivery_count;
        }

        // link credit
        //
        // Only the receiver can independently choose a value for this field. The sender’s
        // value MUST always be maintained in such a way as to match the delivery-limit
        // identified by the receiver.

        // available
        //
        // The receiver’s value is calculated
        // based on the last known value from the sender and any subsequent incoming
        // messages received. The sender MAY transfer messages even if the available variable
        // is zero. If this happens, the receiver MUST maintain a floor of zero in its
        // calculation of the value of available.
        if let Some(available) = flow.available {
            state.available = available;
        }

        // drain
        //
        // The drain flag indicates how the sender SHOULD behave when insufficient messages
        // are available to consume the current link-credit. If set, the sender will (after
        // sending all available messages) advance the delivery-count as much as possible,
        // consuming all link-credit, and send the flow state to the receiver. Only the
        // receiver can independently modify this field. The sender’s value is always the
        // last known value indicated by the receiver.

        match flow.echo {
            true => Some(state.as_link_flow(output_handle, false)),
            false => None,
        }
    }
}

impl<R> LinkFlowState<R> {
    pub async fn drain(&self) -> bool {
        self.lock.read().await.drain
    }

    // pub async fn drain_mut(&self, f: impl Fn(bool) -> bool) {
    //     let mut guard = self.lock.write().await;
    //     let new = f(guard.drain);
    //     guard.drain = new;
    // }

    pub async fn initial_delivery_count(&self) -> SequenceNo {
        self.lock.read().await.initial_delivery_count
    }

    pub async fn initial_delivery_count_mut(&self, f: impl Fn(u32) -> u32) {
        let mut guard = self.lock.write().await;
        let new = f(guard.initial_delivery_count);
        guard.initial_delivery_count = new;
    }

    pub async fn delivery_count(&self) -> SequenceNo {
        self.lock.read().await.delivery_count
    }

    pub async fn delivery_count_mut(&self, f: impl Fn(u32) -> u32) {
        let mut guard = self.lock.write().await;
        let new = f(guard.delivery_count);
        guard.delivery_count = new;
    }

    /// This is async because it is protected behind an async RwLock
    pub async fn properties(&self) -> Option<Fields> {
        self.lock.read().await.properties.clone()
    }
}

impl LinkFlowState<role::Receiver> {
    /// Consume one link credit if available. Returns an error if there is
    /// not enough link credit
    pub async fn consume(&self, count: u32) -> Result<(), super::Error> {
        let mut state = self.lock.write().await;
        if state.link_credit < count {
            Err(super::Error::TransferLimitExceeded)
        } else {
            state.delivery_count += count;
            state.link_credit -= count;
            Ok(())
        }
    }
}

// pub type UnsettledMap<M> = BTreeMap<[u8; 4], M>;
pub(crate) type UnsettledMap<M> = BTreeMap<DeliveryTag, M>;

#[async_trait]
impl ProducerState for Arc<LinkFlowState<role::Sender>> {
    type Item = (LinkFlow, OutputHandle);
    // If echo is requested, a Some(LinkFlow) will be returned
    type Outcome = Option<LinkFlow>;

    #[inline]
    async fn update_state(&mut self, (flow, output_handle): Self::Item) -> Self::Outcome {
        self.on_incoming_flow(flow, output_handle).await
    }
}

impl Producer<Arc<LinkFlowState<role::Sender>>> {
    pub(crate) async fn on_incoming_flow(
        &mut self,
        flow: LinkFlow,
        output_handle: OutputHandle,
    ) -> Option<LinkFlow> {
        self.produce((flow, output_handle)).await
    }
}

// pub enum SenderPermit {
//     Send,
//     Drain,
// }

#[async_trait]
impl Consume for SenderFlowState {
    type Item = u32;
    type Outcome = ();

    /// Increment delivery count and decrement link_credit. Wait asynchronously
    /// if there is not enough credit
    async fn consume(&mut self, item: Self::Item) -> Self::Outcome {
        loop {
            match consume_link_credit(&self.state().lock, item).await {
                Ok(action) => return action,
                Err(_) => self.notifier.notified().await,
            }
        }
    }
}

impl TryConsume for SenderFlowState {
    type Error = SenderTryConsumeError;

    fn try_consume(&mut self, item: Self::Item) -> Result<Self::Outcome, Self::Error> {
        let mut state = self.state().lock.try_write()?;
        if state.link_credit < item {
            Err(Self::Error::InsufficientCredit)
        } else {
            state.delivery_count += item;
            state.link_credit -= item;
            // Ok(SenderPermit::Send)
            Ok(())
        }
    }
}

async fn consume_link_credit(lock: &RwLock<LinkFlowStateInner>, count: u32) -> Result<(), ()> {
    // TODO: Is is worth splitting into a read and then write?
    let mut state = lock.write().await;

    if state.link_credit < count {
        Err(())
    } else {
        state.delivery_count += count;
        state.link_credit -= count;
        // Ok(SenderPermit::Send)
        Ok(())
    }
}
