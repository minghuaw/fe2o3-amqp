//! Helper types differentiating message delivery

use fe2o3_amqp_types::{
    definitions::{DeliveryNumber, DeliveryTag, Handle, MessageFormat, ReceiverSettleMode},
    messaging::{
        message::Body, Accepted, AmqpSequence, AmqpValue, Data, DeliveryState, Message, Outcome,
        MESSAGE_FORMAT,
    },
    primitives::Binary,
};
use futures_util::FutureExt;
use pin_project_lite::pin_project;
use std::{future::Future, marker::PhantomData, task::Poll};
use tokio::sync::oneshot::{self, error::RecvError};

use crate::{
    endpoint::Settlement,
    util::{DeliveryInfo, Uninitialized},
};
use crate::{util::AsDeliveryState, Payload};

use super::{BodyError, LinkStateError, SendError};

/// Reserved for receiver side
#[derive(Debug)]
pub struct Delivery<T> {
    /// Verify whether this message is bound to a link
    pub(crate) link_output_handle: Handle,
    pub(crate) delivery_id: DeliveryNumber,
    pub(crate) delivery_tag: DeliveryTag,

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

    /// Consume the delivery into the message
    pub fn into_message(self) -> Message<T> {
        self.message
    }

    /// Get a reference to the message body
    pub fn body(&self) -> &Body<T> {
        &self.message.body
    }

    /// Consume the delivery into the message body section
    pub fn into_body(self) -> Body<T> {
        self.message.body
    }

    /// Consume the delivery into the body if the body is an [`AmqpValue`].
    /// An error will be returned if the body isnot an [`AmqpValue`]
    pub fn try_into_value(self) -> Result<T, BodyError> {
        match self.into_body() {
            Body::Value(AmqpValue(value)) => Ok(value),
            Body::Data(_) => Err(BodyError::IsData),
            Body::Sequence(_) => Err(BodyError::IsSequence),
            Body::Nothing => Err(BodyError::IsNothing),
        }
    }

    /// Consume the delivery into the body if the body is an [`Data`].
    /// An error will be returned if the body isnot an [`Data`]
    pub fn try_into_data(self) -> Result<Binary, BodyError> {
        match self.into_body() {
            Body::Data(Data(data)) => Ok(data),
            Body::Value(_) => Err(BodyError::IsValue),
            Body::Sequence(_) => Err(BodyError::IsSequence),
            Body::Nothing => Err(BodyError::IsNothing),
        }
    }

    /// Consume the delivery into the body if the body is an [`AmqpSequence`].
    /// An error will be returned if the body isnot an [`AmqpSequence`]
    pub fn try_into_sequence(self) -> Result<Vec<T>, BodyError> {
        match self.into_body() {
            Body::Data(_) => Err(BodyError::IsData),
            Body::Sequence(AmqpSequence(sequence)) => Ok(sequence),
            Body::Value(_) => Err(BodyError::IsValue),
            Body::Nothing => Err(BodyError::IsNothing),
        }
    }

    /// Get a reference to the delivery body if the body is an [`AmqpValue`].
    /// An error will be returned if the body isnot an [`AmqpValue`]
    pub fn try_as_value(&self) -> Result<&T, BodyError> {
        match self.body() {
            Body::Value(AmqpValue(value)) => Ok(value),
            Body::Data(_) => Err(BodyError::IsData),
            Body::Sequence(_) => Err(BodyError::IsSequence),
            Body::Nothing => Err(BodyError::IsNothing),
        }
    }

    /// Get a reference to the delivery body if the body is an [`Data`].
    /// An error will be returned if the body isnot an [`Data`]
    pub fn try_as_data(&self) -> Result<&Binary, BodyError> {
        match self.body() {
            Body::Data(Data(data)) => Ok(data),
            Body::Value(_) => Err(BodyError::IsValue),
            Body::Sequence(_) => Err(BodyError::IsSequence),
            Body::Nothing => Err(BodyError::IsNothing),
        }
    }

    /// Get a reference to the delivery body if the body is an [`AmqpSequence`].
    /// An error will be returned if the body isnot an [`AmqpSequence`]
    pub fn try_as_sequence(&self) -> Result<&Vec<T>, BodyError> {
        match self.body() {
            Body::Data(_) => Err(BodyError::IsData),
            Body::Sequence(AmqpSequence(sequence)) => Ok(sequence),
            Body::Value(_) => Err(BodyError::IsValue),
            Body::Nothing => Err(BodyError::IsNothing),
        }
    }
}

// TODO: Vec doesnt implement display trait
impl<T: std::fmt::Display> std::fmt::Display for Delivery<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.message.body {
            Body::Data(data) => write!(f, "{}", data),
            Body::Sequence(seq) => write!(f, "{}", seq),
            Body::Value(val) => write!(f, "{}", val),
            Body::Nothing => write!(f, "Nothing"),
        }
    }
}

impl<T> Delivery<T> {
    #[cfg(all(feature = "transaction", feature = "acceptor"))]
    pub(crate) fn into_info(self) -> DeliveryInfo {
        DeliveryInfo {
            delivery_id: self.delivery_id,
            delivery_tag: self.delivery_tag,
            rcv_settle_mode: self.rcv_settle_mode,
        }
    }

    pub(crate) fn clone_info(&self) -> DeliveryInfo {
        DeliveryInfo {
            delivery_id: self.delivery_id.clone(),
            delivery_tag: self.delivery_tag.clone(),
            rcv_settle_mode: self.rcv_settle_mode.clone(),
        }
    }
}

/// A type representing the delivery before sending
///
/// This allows pre-setting a message as settled.
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
#[derive(Debug)]
pub struct Sendable<T> {
    pub(crate) message: Message<T>,
    pub(crate) message_format: MessageFormat, // TODO: The message format defined in spec is 0
    pub(crate) settled: Option<bool>,
    // pub(crate) batchable: bool,
}

impl Sendable<Uninitialized> {
    /// Creates a builder for [`Sendable`]
    pub fn builder() -> Builder<Uninitialized> {
        Builder::new()
    }
}

impl<T> From<T> for Sendable<T>
where
    T: Into<Message<T>>,
{
    fn from(value: T) -> Self {
        Self {
            message: value.into(),
            message_format: MESSAGE_FORMAT,
            settled: None,
        }
    }
}

impl<T> From<Message<T>> for Sendable<T> {
    fn from(message: Message<T>) -> Self {
        Self {
            message,
            message_format: MESSAGE_FORMAT,
            settled: None,
        }
    }
}

impl<T> From<Body<T>> for Sendable<T> {
    fn from(body: Body<T>) -> Self {
        let message = Message {
            header: None,
            delivery_annotations: None,
            message_annotations: None,
            properties: None,
            application_properties: None,
            body,
            footer: None,
        };
        Self {
            message,
            message_format: 0,
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
    payload: Payload,
    state: Option<DeliveryState>,
    sender: oneshot::Sender<Option<DeliveryState>>,
}

impl UnsettledMessage {
    pub fn new(payload: Payload, sender: oneshot::Sender<Option<DeliveryState>>) -> Self {
        Self {
            payload,
            state: None,
            sender,
        }
    }

    pub fn state(&self) -> &Option<DeliveryState> {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut Option<DeliveryState> {
        &mut self.state
    }

    pub fn payload(&self) -> &Payload {
        &self.payload
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

    fn as_delivery_state_mut(&mut self) -> &mut Option<DeliveryState> {
        &mut self.state
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

pub(crate) trait FromPreSettled {
    fn from_settled() -> Self;
}

pub(crate) trait FromDeliveryState {
    fn from_none() -> Self;

    fn from_delivery_state(state: DeliveryState) -> Self;
}

pub(crate) trait FromOneshotRecvError {
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
