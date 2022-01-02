use bytes::Bytes;
use fe2o3_amqp_types::{
    definitions::MessageFormat,
    messaging::{DeliveryState, Message, Received},
};
use tokio::sync::oneshot;

use crate::util::Uninitialized;

/// Reserved for receiver side
pub struct Delivery {}

/// TODO: Add a crate level pub field to Delivery for resuming link?
#[derive(Debug)]
pub struct Sendable {
    pub(crate) message: Message,
    pub(crate) message_format: MessageFormat,
    pub(crate) settled: Option<bool>,
    // pub(crate) batchable: bool,
}

impl Sendable {
    pub fn builder() -> Builder<Uninitialized> {
        Builder::new()
    }
}

impl<T> From<T> for Sendable
where
    T: Into<Message>,
{
    fn from(value: T) -> Self {
        Self {
            message: value.into(),
            message_format: 0,
            settled: None,
        }
    }
}

pub struct Builder<T> {
    pub message: T,
    pub message_format: MessageFormat,
    pub settled: Option<bool>,
    pub batchable: bool,
}

impl Builder<Uninitialized> {
    pub fn new() -> Self {
        Self {
            message: Uninitialized {},
            message_format: 0,
            settled: None,
            batchable: false,
        }
    }
}

impl<T> Builder<T> {
    pub fn message(self, message: impl Into<Message>) -> Builder<Message> {
        Builder {
            message: message.into(),
            message_format: self.message_format,
            settled: self.settled,
            batchable: self.batchable,
        }
    }

    pub fn message_format(mut self, message_format: impl Into<MessageFormat>) -> Self {
        self.message_format = message_format.into();
        self
    }

    pub fn settled(mut self, settled: impl Into<Option<bool>>) -> Self {
        self.settled = settled.into();
        self
    }
}

impl Builder<Message> {
    pub fn build(self) -> Sendable {
        Sendable {
            message: self.message,
            message_format: self.message_format,
            settled: self.settled,
            // batchable: self.batchable,
        }
    }
}

impl From<Builder<Message>> for Sendable {
    fn from(builder: Builder<Message>) -> Self {
        builder.build()
    }
}

pub struct UnsettledMessage {
    payload: Bytes,
    state: DeliveryState,
    sender: oneshot::Sender<DeliveryState>,
}

impl UnsettledMessage {
    pub fn new(payload: Bytes, sender: oneshot::Sender<DeliveryState>) -> Self {
        // Assume needing to resend from the beginning unless there is further
        // update from the remote peer
        let received = Received {
            section_number: 0,
            section_offset: 0,
        };

        Self {
            payload,
            state: DeliveryState::Received(received),
            sender,
        }
    }

    pub fn state(&self) -> &DeliveryState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut DeliveryState {
        &mut self.state
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

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

/// A future for delivery that can be `await`ed for the settlement
/// from receiver
pub struct DeliveryFut {
    message: Message,
    outcome: oneshot::Receiver<DeliveryState>,
}
