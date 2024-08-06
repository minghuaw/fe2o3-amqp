//! Link state and link flow state

use std::{marker::PhantomData, sync::Arc};

use fe2o3_amqp_types::definitions::{Fields, SequenceNo};
use parking_lot::RwLock;

use crate::{
    endpoint::{LinkFlow, OutputHandle},
    util::{Consume, ProducerState},
};

use super::{role, ReceiverTransferError, SenderFlowState};

/// Link state.
///
/// There is no official definition of the link state in the specification
#[derive(Debug)]
pub enum LinkState {
    /// The initial state after initialization
    Unattached,

    /// An attach frame has been sent
    AttachSent,

    /// An attach has been sent but with incomplete unsettled
    IncompleteAttachSent,

    /// An attach frame has been received
    AttachReceived,

    /// An attach frame has been received with incomplete unsettled
    IncompleteAttachReceived,

    /// The link is attached
    Attached,

    /// The two endpoints has exchanged Attach frames but at least one of them is incomplete
    IncompleteAttachExchanged,

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
    pub fn as_link_flow(&self, output_handle: OutputHandle, echo: bool, include_properties: bool) -> LinkFlow {
        let properties = if include_properties {
            self.properties.clone()
        } else {
            None
        };

        LinkFlow {
            handle: output_handle.into(),
            delivery_count: Some(self.delivery_count),
            link_credit: Some(self.link_credit),
            available: Some(self.available),
            drain: self.drain,
            echo,
            properties,
        }
    }
}

/// The Sender and Receiver handle link flow control differently
#[derive(Debug)]
pub(crate) struct LinkFlowState<R> {
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

impl LinkFlowState<role::SenderMarker> {
    pub(crate) fn sender(inner: LinkFlowStateInner) -> Self {
        Self::new(inner)
    }
}

impl LinkFlowState<role::ReceiverMarker> {
    pub(crate) fn receiver(inner: LinkFlowStateInner) -> Self {
        Self::new(inner)
    }
}

impl LinkFlowState<role::SenderMarker> {
    /// Handles incoming Flow frame
    ///
    /// If an echo (reply with the local flow state) is requested, return an `Ok(Some(Flow))`,
    /// otherwise, return a `Ok(None)`
    #[inline]
    pub(crate) fn on_incoming_flow(
        &self,
        flow: LinkFlow,
        output_handle: OutputHandle,
    ) -> Option<LinkFlow> {
        let mut state = self.lock.write();

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
            let link_credit = delivery_count_rcv
                .saturating_add(link_credit_rcv)
                .saturating_sub(state.delivery_count);
            state.link_credit = link_credit;
        }

        // available
        //
        // The available variable is controlled by the sender, and indicates to the receiver,
        // that the sender could make use of the indicated amount of link-credit. Only the
        // sender can indepen- dently modify this field.

        // The drain flag indicates how the sender SHOULD behave when insufficient messages
        // are available to consume the current link-credit. If set, the sender will (after
        // sending all available messages) advance the delivery-count as much as possible,
        // consuming all link-credit, and send the flow state to the receiver. Only the
        // receiver can independently modify this field. The sender’s value is always the
        // last known value indicated by the receiver.
        state.drain = flow.drain;
        if flow.drain {
            state.delivery_count = state.delivery_count.wrapping_add(state.link_credit);
            state.link_credit = 0;

            return Some(state.as_link_flow(output_handle, false, false));
        }

        match flow.echo {
            // Should avoid constant ping-pong
            true => Some(state.as_link_flow(output_handle, false, false)),
            false => None,
        }
    }
}

impl LinkFlowState<role::ReceiverMarker> {
    #[inline]
    pub(crate) fn on_incoming_flow(
        &self,
        flow: LinkFlow,
        output_handle: OutputHandle,
    ) -> Option<LinkFlow> {
        let mut state = self.lock.write();

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
            true => Some(state.as_link_flow(output_handle, false, false)),
            false => None,
        }
    }
}

impl<R> LinkFlowState<R> {
    pub fn link_credit(&self) -> u32 {
        self.lock.read().link_credit
    }

    pub fn drain(&self) -> bool {
        self.lock.read().drain
    }

    pub fn initial_delivery_count(&self) -> SequenceNo {
        self.lock.read().initial_delivery_count
    }

    pub fn initial_delivery_count_mut(&self, f: impl Fn(u32) -> u32) {
        let mut guard = self.lock.write();
        let new = f(guard.initial_delivery_count);
        guard.initial_delivery_count = new;
    }

    pub fn delivery_count_mut(&self, f: impl Fn(u32) -> u32) {
        let mut guard = self.lock.write();
        let new = f(guard.delivery_count);
        guard.delivery_count = new;
    }

    /// This is async because it is protected behind an async RwLock
    pub fn properties(&self) -> Option<Fields> {
        self.lock.read().properties.clone()
    }
}

impl LinkFlowState<role::ReceiverMarker> {
    /// Consume one link credit if available. Returns an error if there is
    /// not enough link credit
    pub fn consume(&self, count: u32) -> Result<(), ReceiverTransferError> {
        let mut state = self.lock.write();
        if state.link_credit < count {
            Err(ReceiverTransferError::TransferLimitExceeded)
        } else {
            state.delivery_count = state.delivery_count.wrapping_add(count);
            state.link_credit = state.link_credit.saturating_sub(count);
            Ok(())
        }
    }
}

impl ProducerState for Arc<LinkFlowState<role::SenderMarker>> {
    type Item = (LinkFlow, OutputHandle);
    // If echo is requested, a Some(LinkFlow) will be returned
    type Outcome = Option<LinkFlow>;

    #[inline]
    fn update_state(&mut self, (flow, output_handle): Self::Item) -> Self::Outcome {
        self.on_incoming_flow(flow, output_handle)
    }
}

struct InsufficientCredit {}

impl Consume for SenderFlowState {
    type Item = u32;
    type Outcome = [u8; 4];

    /// Increment delivery count and decrement link_credit. Wait asynchronously
    /// if there is not enough credit
    ///
    /// # Cancel safety
    ///
    /// `Notify` itself is not cancel safe in the way that it would lose its place in the queue.
    /// However, since there can be only one consumer for a producer, losing the place in the queue
    /// does not have any effect. Thus, this IS cancel safe.
    async fn consume(&mut self, item: Self::Item) -> Self::Outcome {
        loop {
            match consume_link_credit(&self.state().lock, item) {
                Ok(outcome) => return outcome,
                Err(_) => self.notifier.notified().await, // **NOT** cancel safe
            }
        }
    }
}

cfg_transaction! {
    impl crate::util::TryConsume for SenderFlowState {
        type Error = super::error::SenderTryConsumeError;

        fn try_consume(&mut self, item: Self::Item) -> Result<Self::Outcome, Self::Error> {
            let mut state = self
                .state()
                .lock
                .try_write()
                .ok_or(super::error::SenderTryConsumeError::TryLockError)?;
            if state.link_credit < item {
                Err(Self::Error::InsufficientCredit)
            } else {
                let tag = state.delivery_count.to_be_bytes();
                state.delivery_count = state.delivery_count.wrapping_add(item);
                state.link_credit = state.link_credit.saturating_sub(item);
                Ok(tag)
            }
        }
    }
}

fn consume_link_credit(
    lock: &RwLock<LinkFlowStateInner>,
    count: u32,
) -> Result<[u8; 4], InsufficientCredit> {
    let mut state = lock.write();

    if state.link_credit < count {
        Err(InsufficientCredit {})
    } else {
        let tag = state.delivery_count.to_be_bytes();
        state.delivery_count = state.delivery_count.wrapping_add(count);
        state.link_credit = state.link_credit.saturating_sub(count);
        Ok(tag)
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use futures_util::poll;
    use tokio::{sync::Notify, time::timeout};

    use crate::{
        endpoint::{LinkFlow, OutputHandle},
        link::{
            role,
            state::{LinkFlowState, LinkFlowStateInner},
            SenderFlowState,
        },
        util::{Consume, Consumer, Produce, Producer},
    };

    macro_rules! assert_pending {
        ($fut:expr) => {
            let fut = Box::pin($fut);
            let poll = poll!(fut);
            assert!(poll.is_pending());
        };
    }

    macro_rules! assert_ready {
        ($fut:expr) => {
            let fut = Box::pin($fut);
            let poll = poll!(fut);
            assert!(poll.is_ready());
        };
    }

    fn create_sender_flow_state_producer_and_consumer() -> (
        Producer<Arc<LinkFlowState<role::SenderMarker>>>,
        SenderFlowState,
    ) {
        let flow_state_inner = LinkFlowStateInner {
            initial_delivery_count: 0,
            delivery_count: 0,
            link_credit: 0,
            available: 0,
            drain: false,
            properties: None,
        };
        let flow_state = Arc::new(LinkFlowState::sender(flow_state_inner));
        let notifier = Arc::new(Notify::new());
        let producer = Producer::new(notifier.clone(), flow_state.clone());
        let consumer = Consumer::new(notifier, flow_state);
        (producer, consumer)
    }

    #[tokio::test]
    async fn test_sender_flow_state_producer_and_consumer() {
        let (mut producer, mut consumer) = create_sender_flow_state_producer_and_consumer();

        // .await on notify
        assert_pending!(consumer.consume(1));

        // .await after notify with zero credit
        let item = (LinkFlow::default(), OutputHandle(0));
        producer.produce(item).await;
        assert_pending!(consumer.consume(1));

        // .await after notify with non-zero credit
        let link_flow = LinkFlow {
            link_credit: Some(2),
            ..Default::default()
        };
        let item = (link_flow, OutputHandle(0));
        producer.produce(item).await;
        assert_ready!(consumer.consume(1));
        assert_ready!(consumer.consume(1));

        // All credits have been consumed already
        assert_pending!(consumer.consume(1));
    }

    #[tokio::test]
    async fn test_spawned_flow_state_producer_and_consumer() {
        let (mut producer, mut consumer) = create_sender_flow_state_producer_and_consumer();

        let handle = tokio::spawn(async move { consumer.consume(1).await });
        let mut fut = Box::pin(handle);
        assert_pending!(&mut fut);

        // .await after notify with non-zero credit
        let link_flow = LinkFlow {
            link_credit: Some(1),
            ..Default::default()
        };
        let item = (link_flow, OutputHandle(0));
        producer.produce(item).await;

        let wait = timeout(Duration::from_millis(500), fut).await;
        assert!(wait.is_ok());
    }

    #[tokio::test]
    async fn test_drop_consume_fut_before_produce() {
        let (mut producer, mut consumer) = create_sender_flow_state_producer_and_consumer();

        // Drop before notify
        let fut = consumer.consume(1);
        let mut pinned = Box::pin(fut);
        assert!(poll!(&mut pinned).is_pending());
        drop(pinned);

        // .await after notify with non-zero credit
        let link_flow = LinkFlow {
            link_credit: Some(2),
            ..Default::default()
        };
        let item = (link_flow, OutputHandle(0));
        producer.produce(item).await;

        // If it is not cancel safe, we cannot consume all 2 credits
        assert_ready!(consumer.consume(1));
        assert_ready!(consumer.consume(1));

        // All credits have been consumed already
        assert_pending!(consumer.consume(1));
    }

    #[tokio::test]
    async fn test_drop_consume_fut_after_produce() {
        let (mut producer, mut consumer) = create_sender_flow_state_producer_and_consumer();

        // Drop before notify
        let fut = consumer.consume(1);
        let mut pinned = Box::pin(fut);
        assert!(poll!(&mut pinned).is_pending());

        // .await after notify with non-zero credit
        let link_flow = LinkFlow {
            link_credit: Some(2),
            ..Default::default()
        };
        let item = (link_flow, OutputHandle(0));
        producer.produce(item).await;

        drop(pinned);

        // If it is not cancel safe, we cannot consume all 2 credits
        assert_ready!(consumer.consume(1));
        assert_ready!(consumer.consume(1));

        // All credits have been consumed already
        assert_pending!(consumer.consume(1));
    }
}
