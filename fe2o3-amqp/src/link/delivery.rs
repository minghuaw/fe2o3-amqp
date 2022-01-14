use bytes::BytesMut;
use fe2o3_amqp_types::{
    definitions::{AmqpError, DeliveryNumber, DeliveryTag, Handle, MessageFormat},
    messaging::{DeliveryState, Message, Received},
};
use futures_util::FutureExt;
use pin_project_lite::pin_project;
use std::{future::Future, task::Poll};
use tokio::sync::oneshot;

use crate::link;
use crate::{endpoint::Settlement, util::Uninitialized};

/// Reserved for receiver side
pub struct Delivery {
    /// Verify whether this message is bound to a link
    pub(crate) link_output_handle: Handle,
    pub(crate) delivery_id: DeliveryNumber,
    pub(crate) delivery_tag: DeliveryTag,
    pub(crate) message: Message,
}

impl Delivery {
    pub fn message(&self) -> &Message {
        &self.message
    }
}

/// TODO: Add a crate level pub field to Delivery for resuming link?
#[derive(Debug)]
pub struct Sendable {
    pub(crate) message: Message,
    pub(crate) message_format: MessageFormat, // TODO: The message format defined in spec is 0
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

#[derive(Debug)]
pub struct UnsettledMessage {
    payload: BytesMut,
    state: DeliveryState,
    sender: oneshot::Sender<DeliveryState>,
}

impl UnsettledMessage {
    pub fn new(payload: BytesMut, sender: oneshot::Sender<DeliveryState>) -> Self {
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

    pub fn payload(&self) -> &BytesMut {
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

impl AsRef<DeliveryState> for UnsettledMessage {
    fn as_ref(&self) -> &DeliveryState {
        &self.state
    }
}

pin_project! {
    /// A future for delivery that can be `await`ed for the settlement
    /// from receiver
    pub struct DeliveryFut {
        #[pin]
        // Reserved for future use on actively sending disposition from Sender
        settlement: Settlement,
    }
}

impl From<Settlement> for DeliveryFut {
    fn from(settlement: Settlement) -> Self {
        Self { settlement }
    }
}

impl Future for DeliveryFut {
    type Output = Result<(), link::Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let mut settlement = this.settlement;

        match &mut *settlement {
            Settlement::Settled => Poll::Ready(Ok(())),
            Settlement::Unsettled {
                delivery_tag: _,
                outcome,
            } => {
                match outcome.poll_unpin(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(result) => {
                        match result {
                            Ok(state) => {
                                let result = match state {
                                    DeliveryState::Accepted(_) | DeliveryState::Received(_) => {
                                        Ok(())
                                    }
                                    DeliveryState::Rejected(rejected) => {
                                        Err(link::Error::Rejected(rejected))
                                    }
                                    DeliveryState::Released(released) => {
                                        Err(link::Error::Released(released))
                                    }
                                    DeliveryState::Modified(modified) => {
                                        Err(link::Error::Modified(modified))
                                    }
                                };
                                Poll::Ready(result)
                            }
                            Err(_) => {
                                // If the sender is dropped, there is likely issues with the connection
                                // or the session, and thus the error should propagate to the user
                                Poll::Ready(Err(link::Error::AmqpError {
                                    condition: AmqpError::IllegalState,
                                    description: Some("Outcome sender is dropped".into()),
                                }))
                            }
                        }
                    }
                }
            }
        }
    }
}
