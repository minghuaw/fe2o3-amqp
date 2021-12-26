mod frame;
use std::sync::Arc;

use async_trait::async_trait;
use fe2o3_amqp_types::definitions::{AmqpError, Fields, SequenceNo};
pub use frame::*;
pub mod builder;
mod error;
pub mod receiver;
pub mod receiver_link;
pub mod sender;
pub mod sender_link;
pub use error::Error;

use futures_util::{Sink, Stream};
pub use receiver::Receiver;
pub use sender::Sender;
use tokio::sync::{mpsc, RwLock};

use crate::{
    endpoint::{self, LinkFlow},
    util::{Constant, Consume, Consumer, Produce, Producer, ProducerState},
};

pub mod type_state {
    pub struct Attached {}

    pub struct Detached {}
}

pub mod role {

    /// Type state for link::builder::Builder
    pub struct Sender {}

    /// Type state for link::builder::Builder
    pub struct Receiver {}
}

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
    // /// A closing detach frame has been sent
    // CloseSent,

    // CloseReceived,

    // Closed,
}

pub struct LinkFlowStateInner {
    pub initial_delivery_count: Constant<SequenceNo>,
    pub delivery_count: SequenceNo, // SequenceNo = u32
    pub link_credit: u32,
    pub avaiable: u32,
    pub drain: bool,
    pub properties: Option<Fields>,
}

impl From<&LinkFlowStateInner> for LinkFlow {
    fn from(state: &LinkFlowStateInner) -> Self {
        LinkFlow {
            delivery_count: Some(state.delivery_count),
            link_credit: Some(state.link_credit),
            available: Some(state.avaiable),
            drain: state.drain,
            echo: false,
        }
    }
}

/// The Sender and Receiver handle link flow control differently
pub enum LinkFlowState {
    Sender(RwLock<LinkFlowStateInner>),
    Receiver(RwLock<LinkFlowStateInner>),
}

impl LinkFlowState {
    /// Handles incoming Flow frame
    ///
    /// TODO: Is a result necessary?
    ///
    /// If an echo (reply with the local flow state) is requested, return an `Ok(Some(Flow))`,
    /// otherwise, return a `Ok(None)`
    #[inline]
    pub(crate) async fn on_incoming_flow(&self, flow: LinkFlow) -> Option<LinkFlow> {
        println!(">>> Debug: LinkFlowState::on_incoming_flow");

        match self {
            LinkFlowState::Sender(lock) => {
                let mut state = lock.write().await;

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
                let delivery_count_rcv = flow.delivery_count.unwrap_or_else(|| {
                    // In the event that the receiver does not yet know the delivery-count,
                    // i.e., delivery-count_rcv is unspecified, the sender MUST assume that
                    // the delivery-count_rcv is the first delivery-count_snd sent from sender
                    // to receiver, i.e., the delivery-count_snd specified in the flow state
                    // carried by the initial attach frame from the sender to the receiver.
                    *state.initial_delivery_count.value()
                });

                if let Some(link_credit_rcv) = flow.link_credit {
                    let link_credit = delivery_count_rcv + link_credit_rcv - state.delivery_count;
                    state.link_credit = link_credit;
                }

                // available
                //
                // The available variable is controlled by the sender, and indicates to the receiver,
                // that the sender could make use of the indicated amount of link-credit. Only the
                // sender can indepen- dently modify this field.

                // drain
                //
                // The drain flag indicates how the sender SHOULD behave when insufficient messages
                // are available to consume the current link-credit. If set, the sender will (after
                // sending all available messages) advance the delivery-count as much as possible,
                // consuming all link-credit, and send the flow state to the receiver. Only the
                // receiver can independently modify this field. The sender’s value is always the
                // last known value indicated by the receiver.
                state.drain = flow.drain;

                match flow.echo {
                    true => Some(LinkFlow::from(&*state)),
                    false => None,
                }
            }
            LinkFlowState::Receiver(lock) => {
                let mut state = lock.write().await;

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
                    state.avaiable = available;
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
                    true => Some(LinkFlow::from(&*state)),
                    false => None,
                }
            }
        }
    }

    pub async fn drain(&self) -> bool {
        match self {
            LinkFlowState::Sender(lock) => lock.read().await.drain,
            LinkFlowState::Receiver(lock) => lock.read().await.drain,
        }
    }

    pub async fn initial_delivery_count(&self) -> SequenceNo {
        match self {
            LinkFlowState::Sender(lock) => *lock.read().await.initial_delivery_count.value(),
            LinkFlowState::Receiver(lock) => *lock.read().await.initial_delivery_count.value(),
        }
    }

    pub async fn delivery_count(&self) -> SequenceNo {
        match self {
            LinkFlowState::Sender(lock) => lock.read().await.delivery_count,
            LinkFlowState::Receiver(lock) => lock.read().await.delivery_count,
        }
    }

    /// This is async because it is protected behind an async RwLock
    pub async fn properties(&self) -> Option<Fields> {
        match self {
            LinkFlowState::Sender(lock) => lock.read().await.properties.clone(),
            LinkFlowState::Receiver(lock) => lock.read().await.properties.clone(),
        }
    }
}

pub struct LinkHandle {
    pub tx: mpsc::Sender<LinkIncomingItem>,
    pub flow_state: Producer<Arc<LinkFlowState>>,
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

#[async_trait]
impl ProducerState for Arc<LinkFlowState> {
    type Item = LinkFlow;
    // If echo is requested, a Some(LinkFlow) will be returned
    type Outcome = Option<LinkFlow>;

    #[inline]
    async fn update_state(&mut self, item: Self::Item) -> Self::Outcome {
        self.on_incoming_flow(item).await
    }
}

impl Producer<Arc<LinkFlowState>> {
    pub async fn on_incoming_flow(&mut self, flow: LinkFlow) -> Option<LinkFlow> {
        self.produce(flow).await
    }
}

#[async_trait]
impl Consume for Consumer<Arc<LinkFlowState>> {
    type Item = u32;
    type Outcome = ();

    async fn consume(&mut self, item: Self::Item) -> Self::Outcome {
        // check whether there is anough credit
        match self.state().as_ref() {
            LinkFlowState::Sender(lock) => {
                // increment delivery count and decrement link_credit
                loop {
                    match consume_link_credit(&lock, item).await {
                        Ok(_) => return (),
                        Err(_) => self.notifier.notified().await,
                    }
                }
            }
            LinkFlowState::Receiver(lock) => {
                todo!()
            }
        }
    }
}

async fn consume_link_credit(lock: &RwLock<LinkFlowStateInner>, count: u32) -> Result<(), ()> {
    let mut state = lock.write().await;
    if state.link_credit < count {
        return Err(());
    } else {
        state.delivery_count += count;
        state.link_credit -= count;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
            let item = LinkFlow::default();
            producer.produce(item).await;
        });

        notified.await;
        println!("wait passed");

        handle.await.unwrap();
    }
}
