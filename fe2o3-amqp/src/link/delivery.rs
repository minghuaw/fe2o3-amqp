//! Helper types differentiating message delivery

use fe2o3_amqp_types::{
    definitions::{DeliveryNumber, DeliveryTag, Handle, MessageFormat},
    messaging::{message::Body, DeliveryState, Message, Outcome, Received},
};
use futures_util::FutureExt;
use pin_project_lite::pin_project;
use std::{future::Future, marker::PhantomData, task::Poll};
use tokio::sync::oneshot::{self, error::RecvError};

use crate::Payload;
use crate::{endpoint::Settlement, util::Uninitialized};

use super::{LinkStateError, SendError};

/// Reserved for receiver side
#[derive(Debug)]
pub struct Delivery<T> {
    /// Verify whether this message is bound to a link
    pub(crate) link_output_handle: Handle,
    pub(crate) delivery_id: DeliveryNumber,
    pub(crate) delivery_tag: DeliveryTag,
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
}

// TODO: Vec doesnt implement display trait
impl<T: std::fmt::Display> std::fmt::Display for Delivery<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.message.body {
            Body::Data(data) => write!(f, "{}", data),
            Body::Sequence(seq) => write!(f, "{}", seq),
            Body::Value(val) => write!(f, "{}", val),
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
            message_format: 0,
            settled: None,
        }
    }
}

impl<T> From<Message<T>> for Sendable<T> {
    fn from(message: Message<T>) -> Self {
        Self {
            message,
            message_format: 0,
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
            message_format: 0,
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
    _payload: Payload,
    state: DeliveryState,
    sender: oneshot::Sender<DeliveryState>,
}

impl UnsettledMessage {
    pub fn new(payload: Payload, sender: oneshot::Sender<DeliveryState>) -> Self {
        // Assume needing to resend from the beginning unless there is further
        // update from the remote peer
        let received = Received {
            section_number: 0,
            section_offset: 0,
        };

        Self {
            _payload: payload,
            state: DeliveryState::Received(received),
            sender,
        }
    }

    // pub fn state(&self) -> &DeliveryState {
    //     &self.state
    // }

    pub fn state_mut(&mut self) -> &mut DeliveryState {
        &mut self.state
    }

    // pub fn payload(&self) -> &Payload {
    //     &self._payload
    // }

    pub fn settle(self) -> Result<(), DeliveryState> {
        self.sender.send(self.state)
    }

    pub fn settle_with_state(self, state: Option<DeliveryState>) -> Result<(), DeliveryState> {
        match state {
            Some(state) => self.sender.send(state),
            None => self.settle(),
        }
    }
}

impl AsRef<DeliveryState> for UnsettledMessage {
    fn as_ref(&self) -> &DeliveryState {
        &self.state
    }
}

impl AsMut<DeliveryState> for UnsettledMessage {
    fn as_mut(&mut self) -> &mut DeliveryState {
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

impl<O> From<Settlement> for DeliveryFut<O> {
    fn from(settlement: Settlement) -> Self {
        Self {
            settlement,
            outcome_marker: PhantomData,
        }
    }
}

pub(crate) trait FromSettled {
    fn from_settled() -> Self;
}

pub(crate) trait FromDeliveryState {
    fn from_delivery_state(state: DeliveryState) -> Self;
}

pub(crate) trait FromOneshotRecvError {
    fn from_oneshot_recv_error(err: RecvError) -> Self;
}

// impl<T, E> FromOneshotRecvError for Result<T, E>
// where
//     E: From<RecvError>,
// {
//     fn from_oneshot_recv_error(err: RecvError) -> Self {
//         Err(E::from(err))
//     }
// }

impl FromOneshotRecvError for SendResult {
    fn from_oneshot_recv_error(_: RecvError) -> Self {
        Err(LinkStateError::IllegalSessionState.into())
    }
}

type SendResult = Result<(), SendError>;

impl FromSettled for SendResult {
    fn from_settled() -> Self {
        Ok(())
    }
}

impl FromDeliveryState for SendResult {
    fn from_delivery_state(state: DeliveryState) -> Self {
        match state {
            DeliveryState::Accepted(_) | DeliveryState::Received(_) => Ok(()),
            DeliveryState::Rejected(rejected) => Err(SendError::Rejected(rejected)),
            DeliveryState::Released(released) => Err(SendError::Released(released)),
            DeliveryState::Modified(modified) => Err(SendError::Modified(modified)),
            #[cfg(feature = "transaction")]
            DeliveryState::Declared(_) => Err(SendError::IllegalDeliveryState),
            #[cfg(feature = "transaction")]
            DeliveryState::TransactionalState(txn_state) => match txn_state.outcome {
                Some(Outcome::Accepted(_)) => Ok(()),
                Some(Outcome::Rejected(value)) => Err(SendError::Rejected(value)),
                Some(Outcome::Released(value)) => Err(SendError::Released(value)),
                Some(Outcome::Modified(value)) => Err(SendError::Modified(value)),
                Some(Outcome::Declared(_)) | None => Err(SendError::IllegalDeliveryState),
            },
        }
    }
}

impl<O> Future for DeliveryFut<O>
where
    O: FromSettled + FromDeliveryState + FromOneshotRecvError,
{
    type Output = O;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let mut settlement = this.settlement;

        match &mut *settlement {
            Settlement::Settled => Poll::Ready(O::from_settled()),
            Settlement::Unsettled {
                _delivery_tag: _,
                outcome,
            } => {
                match outcome.poll_unpin(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(result) => {
                        match result {
                            Ok(state) => Poll::Ready(O::from_delivery_state(state)),
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
