//! Helper types differentiating message delivery

use fe2o3_amqp_types::{
    definitions::{DeliveryNumber, DeliveryTag, Handle, MessageFormat, ReceiverSettleMode},
    messaging::{Accepted, DeliveryState, Message, Outcome, SerializableBody, MESSAGE_FORMAT},
    primitives::BinaryRef,
};
use futures_util::FutureExt;
use pin_project_lite::pin_project;
use std::{future::Future, marker::PhantomData, task::Poll};
use tokio::sync::oneshot::{self, error::RecvError};

use crate::{
    endpoint::Settlement,
    util::{Sealed, Uninitialized},
};
use crate::{util::AsDeliveryState, Payload};

use super::{LinkStateError, SendError};

/// Delivery information that is needed for disposing a message
#[derive(Clone)]
pub struct DeliveryInfo {
    /// Delivery ID carried by the transfer frame
    pub(crate) delivery_id: DeliveryNumber,

    /// Delivery Tag carried by the transfer frame
    pub(crate) delivery_tag: DeliveryTag,

    /// Receiver settle mode that is carried by the transfer frame
    pub(crate) rcv_settle_mode: Option<ReceiverSettleMode>,

    pub(crate) _sealed: Sealed,
}

impl DeliveryInfo {
    /// Get the delivery ID carried by the transfer frame
    pub fn delivery_id(&self) -> DeliveryNumber {
        self.delivery_id
    }

    /// get the delivery Tag carried by the transfer frame
    pub fn delivery_tag(&self) -> &DeliveryTag {
        &self.delivery_tag
    }

    /// Get the receiver settle mode that is carried by the transfer frame
    pub fn rcv_settle_mode(&self) -> &Option<ReceiverSettleMode> {
        &self.rcv_settle_mode
    }
}

impl std::fmt::Debug for DeliveryInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeliveryInfo")
            .field("delivery_id", &self.delivery_id)
            .field("delivery_tag", &self.delivery_tag)
            .field("rcv_settle_mode", &self.rcv_settle_mode)
            .finish()
    }
}

impl<T> From<Delivery<T>> for DeliveryInfo {
    fn from(delivery: Delivery<T>) -> Self {
        Self {
            delivery_id: delivery.delivery_id,
            delivery_tag: delivery.delivery_tag,
            rcv_settle_mode: delivery.rcv_settle_mode,
            _sealed: Sealed {},
        }
    }
}

impl<T> From<&Delivery<T>> for DeliveryInfo {
    fn from(delivery: &Delivery<T>) -> Self {
        Self {
            delivery_id: delivery.delivery_id,
            delivery_tag: delivery.delivery_tag.clone(),
            rcv_settle_mode: delivery.rcv_settle_mode.clone(),
            _sealed: Sealed {},
        }
    }
}

/// Reserved for receiver side
#[derive(Debug)]
pub struct Delivery<T> {
    /// Verify whether this message is bound to a link
    pub(crate) link_output_handle: Handle,
    pub(crate) delivery_id: DeliveryNumber,
    pub(crate) delivery_tag: DeliveryTag,

    pub(crate) message_format: Option<MessageFormat>,
    pub(crate) rcv_settle_mode: Option<ReceiverSettleMode>,

    pub(crate) message: Message<T>,
}

impl<T> Delivery<T> {
    /// Get the link output handle
    pub fn handle(&self) -> &Handle {
        &self.link_output_handle
    }

    /// Get the message
    pub fn message(&self) -> &Message<T> {
        &self.message
    }

    /// Get the delivery ID
    pub fn delivery_id(&self) -> &DeliveryNumber {
        &self.delivery_id
    }

    /// Get the delivery tag
    pub fn delivery_tag(&self) -> &DeliveryTag {
        &self.delivery_tag
    }

    /// Get the message format
    pub fn message_format(&self) -> &Option<MessageFormat> {
        &self.message_format
    }

    /// Consume the delivery into the message
    pub fn into_message(self) -> Message<T> {
        self.message
    }

    /// Get a reference to the message body
    pub fn body(&self) -> &T {
        &self.message.body
    }

    /// Consume the delivery into the message body section
    pub fn into_body(self) -> T {
        self.message.body
    }

    /// Consume the delivery into the delivery info and message.
    /// The message format will be lost.
    pub fn into_parts(self) -> (DeliveryInfo, Message<T>) {
        (
            DeliveryInfo {
                delivery_id: self.delivery_id,
                delivery_tag: self.delivery_tag,
                rcv_settle_mode: self.rcv_settle_mode,
                _sealed: Sealed {},
            },
            self.message,
        )
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Delivery<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Delivery {{ delivery_id: {}, delivery_tag: {:#x}, message.body: {} }}",
            self.delivery_id(),
            BinaryRef(&self.delivery_tag()[..]),
            self.body()
        )
    }
}

/// A type representing the delivery before sending
///
/// This allows pre-setting a message as settled if the sender's settle mode is set
/// to `SenderSettleMode::Mixed`.
///
/// # Example
///
/// ```rust, ignore
/// let sendable = Sendable::builder()
///     .message("hello world")
///     .settled(true)
///     .build();
/// sender.send(sendable).await.unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct Sendable<T> {
    /// The message to send
    pub message: Message<T>,

    /// Please see page 82 of the AMQP 1.0 core specification
    pub message_format: MessageFormat,

    /// Whether the message will be sent pre-settled
    ///
    /// Please note that this field will be neglected if the negotiated
    /// sender settle mode is NOT equal to `SenderSettleMode::Mixed`
    pub settled: Option<bool>,
}

impl Sendable<Uninitialized> {
    /// Creates a builder for [`Sendable`]
    pub fn builder() -> Builder<Uninitialized> {
        Builder::new()
    }
}

impl<T, U> From<T> for Sendable<U>
where
    T: Into<Message<U>>,
    U: SerializableBody,
{
    fn from(value: T) -> Self {
        Self {
            message: value.into(),
            message_format: MESSAGE_FORMAT,
            settled: None,
        }
    }
}

/// A builder for [`Sendable`]
#[derive(Debug)]
pub struct Builder<T> {
    /// The message to send
    pub message: T,

    /// Message format.
    ///
    /// See 2.8.11 Message Format in the AMQP1.0 specification
    pub message_format: MessageFormat,

    /// Indicates whether the message is considered settled by the sender
    pub settled: Option<bool>,
    // pub batchable: bool,
}

impl Default for Builder<Uninitialized> {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder<Uninitialized> {
    /// Creates a new builder for [`Sendable`]
    pub fn new() -> Self {
        Self {
            message: Uninitialized {},
            message_format: MESSAGE_FORMAT,
            settled: None,
            // batchable: false,
        }
    }
}

impl<State> Builder<State> {
    /// The message to send
    pub fn message<T>(self, message: impl Into<Message<T>>) -> Builder<Message<T>> {
        Builder {
            message: message.into(),
            message_format: self.message_format,
            settled: self.settled,
            // batchable: self.batchable,
        }
    }

    /// Message format.
    ///
    /// See 2.8.11 Message Format in the AMQP1.0 specification
    pub fn message_format(mut self, message_format: MessageFormat) -> Self {
        self.message_format = message_format;
        self
    }

    /// Indicates whether the message is considered settled by the sender
    pub fn settled(mut self, settled: impl Into<Option<bool>>) -> Self {
        self.settled = settled.into();
        self
    }
}

impl<T> Builder<Message<T>> {
    /// Builds a [`Sendable`]
    pub fn build(self) -> Sendable<T> {
        Sendable {
            message: self.message,
            message_format: self.message_format,
            settled: self.settled,
            // batchable: self.batchable,
        }
    }
}

impl<T> From<Builder<Message<T>>> for Sendable<T> {
    fn from(builder: Builder<Message<T>>) -> Self {
        builder.build()
    }
}

/// An unsettled message stored in the Sender's unsettled map
#[derive(Debug)]
pub(crate) struct UnsettledMessage {
    pub(crate) payload: Payload,
    pub(crate) state: Option<DeliveryState>,
    pub(crate) message_format: u32,
    pub(crate) sender: oneshot::Sender<Option<DeliveryState>>,
}

impl UnsettledMessage {
    pub fn new(
        payload: Payload,
        state: Option<DeliveryState>,
        message_format: u32,
        sender: oneshot::Sender<Option<DeliveryState>>,
    ) -> Self {
        Self {
            payload,
            state,
            message_format,
            sender,
        }
    }

    pub fn settle(self) -> Result<(), Option<DeliveryState>> {
        self.sender.send(self.state)
    }

    pub fn settle_with_state(
        self,
        state: Option<DeliveryState>,
    ) -> Result<(), Option<DeliveryState>> {
        self.sender.send(state)
    }
}

impl AsDeliveryState for UnsettledMessage {
    fn as_delivery_state(&self) -> &Option<DeliveryState> {
        &self.state
    }
}

pin_project! {
    /// A future for delivery that can be `.await`ed for the settlement
    /// from receiver
    pub struct DeliveryFut<O> {
        #[pin]
        // Reserved for future use on actively sending disposition from Sender
        settlement: Settlement,
        outcome_marker: PhantomData<O>
    }
}

impl<O> DeliveryFut<O> {
    /// Get the delivery tag
    pub fn delivery_tag(&self) -> &DeliveryTag {
        match &self.settlement {
            Settlement::Settled(delivery_tag) => delivery_tag,
            Settlement::Unsettled {
                delivery_tag,
                outcome: _,
            } => delivery_tag,
        }
    }
}

impl<O> From<Settlement> for DeliveryFut<O> {
    fn from(settlement: Settlement) -> Self {
        Self {
            settlement,
            outcome_marker: PhantomData,
        }
    }
}

/// This trait defines how to interprete a pre-settled delivery
///
/// This is public for compatibility with rust versions <= 1.58.0
pub trait FromPreSettled {
    /// how to interprete a pre-settled delivery
    fn from_settled() -> Self;
}

/// This trait defines how to interprete a DeliveryState
///
/// This is public for compatibility with rust versions <= 1.58.0
pub trait FromDeliveryState {
    /// how to interprete a DeliveryState when `None` is found
    fn from_none() -> Self;

    /// how to interprete a DeliveryState
    fn from_delivery_state(state: DeliveryState) -> Self;
}

/// This trait defines how to interprete `tokio::sync::oneshot::error::RecvError`
///
/// This is public for compatibility with rust versions <= 1.58.0
pub trait FromOneshotRecvError {
    /// how to interprete `tokio::sync::oneshot::error::RecvError`
    fn from_oneshot_recv_error(err: RecvError) -> Self;
}

impl FromOneshotRecvError for SendResult {
    fn from_oneshot_recv_error(_: RecvError) -> Self {
        Err(LinkStateError::IllegalSessionState.into())
    }
}

pub(crate) type SendResult = Result<Outcome, SendError>;

impl FromPreSettled for SendResult {
    fn from_settled() -> Self {
        Ok(Outcome::Accepted(Accepted {}))
    }
}

impl FromDeliveryState for SendResult {
    fn from_none() -> Self {
        Err(SendError::IllegalDeliveryState)
    }

    fn from_delivery_state(state: DeliveryState) -> Self {
        match state {
            // DeliveryState::Accepted(accepted) | DeliveryState::Received(_) => Ok(accepted),
            // DeliveryState::Rejected(rejected) => Err(SendError::Rejected(rejected)),
            // DeliveryState::Released(released) => Err(SendError::Released(released)),
            // DeliveryState::Modified(modified) => Err(SendError::Modified(modified)),
            DeliveryState::Accepted(accepted) => Ok(Outcome::Accepted(accepted)),
            DeliveryState::Rejected(rejected) => Ok(Outcome::Rejected(rejected)),
            DeliveryState::Released(released) => Ok(Outcome::Released(released)),
            DeliveryState::Modified(modified) => Ok(Outcome::Modified(modified)),
            DeliveryState::Received(_) => Err(SendError::NonTerminalDeliveryState),
            #[cfg(feature = "transaction")]
            DeliveryState::Declared(_) | DeliveryState::TransactionalState(_) => {
                Err(SendError::IllegalDeliveryState)
            }
        }
    }
}

impl<O> Future for DeliveryFut<O>
where
    O: FromPreSettled + FromDeliveryState + FromOneshotRecvError,
{
    type Output = O;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let mut settlement = this.settlement;

        match &mut *settlement {
            Settlement::Settled(_) => Poll::Ready(O::from_settled()),
            Settlement::Unsettled {
                delivery_tag: _,
                outcome,
            } => {
                match outcome.poll_unpin(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(result) => {
                        match result {
                            Ok(Some(state)) => Poll::Ready(O::from_delivery_state(state)),
                            Ok(None) => Poll::Ready(O::from_none()),
                            Err(err) => {
                                // If the sender is dropped, there is likely issues with the connection
                                // or the session, and thus the error should propagate to the user
                                Poll::Ready(O::from_oneshot_recv_error(err))
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp_types::{
        messaging::{AmqpValue, Body, Data, Message},
        primitives::Binary,
    };

    use crate::Sendable;

    struct Foo {}

    impl From<Foo> for Message<Data> {
        fn from(_: Foo) -> Self {
            Message::builder().data(Binary::from("Foo")).build()
        }
    }

    #[test]
    fn test_from_primitive_into_sendable() {
        let value = false;
        let sendable = Sendable::from(value);
        assert_eq!(sendable.message.body, AmqpValue(false));

        // let mut map = std::collections::BTreeMap::new();
        // map.insert(String::from("hello"), String::from("world"));
        // let sendable = Sendable::from(map);
    }

    #[test]
    fn test_from_body_into_sendable() {
        let body = Body::Value(AmqpValue(1.23456_f64));
        let sendable = Sendable::from(body);
        assert_eq!(sendable.message.body, Body::Value(AmqpValue(1.23456_f64)));
    }

    #[test]
    fn test_from_message_into_sendable() {
        let message = Message::builder().value(5671_u32).build();
        let sendable = Sendable::from(message);
        assert_eq!(sendable.message.body, AmqpValue(5671_u32));
    }

    #[test]
    fn test_from_custom_type_into_sendable() {
        let value = Foo {};
        let sendable = Sendable::from(value);
        assert_eq!(sendable.message.body, Data(Binary::from("Foo")));
    }
}
