//! Defines traits for link implementations

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{
        DeliveryNumber, DeliveryTag, Error, MessageFormat, ReceiverSettleMode,
        Role, SequenceNo,
    },
    messaging::DeliveryState,
    performatives::{Attach, Detach, Transfer},
};
use futures_util::{Future, Sink};

use crate::{
    link::{delivery::Delivery, LinkFrame, },
    Payload, 
};

use super::{OutputHandle, Settlement};

#[async_trait]
pub(crate) trait LinkDetach {
    type DetachError: Send;

    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::DetachError>;

    async fn send_detach<W>(
        &mut self,
        writer: &mut W,
        closed: bool,
        error: Option<Error>,
    ) -> Result<(), Self::DetachError>
    where
        W: Sink<LinkFrame> + Send + Unpin;
}

#[async_trait]
pub(crate) trait LinkAttach {
    type AttachError: Send;

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::AttachError>;

    async fn send_attach<W>(&mut self, writer: &mut W) -> Result<(), Self::AttachError>
    where
        W: Sink<LinkFrame> + Send + Unpin;
}

#[async_trait]
pub(crate) trait LinkAttachAcceptorExt: LinkAttach {
    async fn on_incoming_attach_as_acceptor(
        &mut self,
        attach: Attach,
    ) -> Result<(), (Self::AttachError, Option<Attach>)>;
}

pub(crate) trait Link: LinkAttach + LinkDetach {
    fn role() -> Role;
}

pub(crate) trait LinkExt: Link {
    type FlowState;
    type Unsettled;
    type Target;

    fn name(&self) -> &str;

    fn output_handle(&self) -> &Option<OutputHandle>;

    fn output_handle_mut(&mut self) -> &mut Option<OutputHandle>;

    fn flow_state(&self) -> &Self::FlowState;

    fn unsettled(&self) -> &Self::Unsettled;

    fn rcv_settle_mode(&self) -> &ReceiverSettleMode;

    fn target(&self) -> &Option<Self::Target>;
}


#[async_trait]
pub(crate) trait SenderLink: Link + LinkExt {
    type Error: Send;

    /// Set and send flow state
    async fn send_flow<W>(
        &mut self,
        writer: &mut W,
        delivery_count: Option<SequenceNo>,
        available: Option<u32>,
        echo: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin;

    /// Send message via transfer frame and return whether the message is already settled
    async fn send_payload<W, Fut>(
        &mut self,
        writer: &mut W,
        detached: Fut,
        payload: Payload,
        message_format: MessageFormat,
        settled: Option<bool>,
        // The delivery state from sender is useful for
        // 1. link resumption
        // 2. transaction
        // The delivery state should be attached on every transfer if specified
        state: Option<DeliveryState>,
        batchable: bool,
    ) -> Result<Settlement, Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin,
        Fut: Future<Output = Option<LinkFrame>> + Send;

    async fn dispose<W>(
        &mut self,
        writer: &mut W,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin;

    async fn batch_dispose<W>(
        &mut self,
        writer: &mut W,
        ids_and_tags: Vec<(DeliveryNumber, DeliveryTag)>,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin;
}

#[async_trait]
pub(crate) trait ReceiverLink: Link + LinkExt {
    type Error: Send;

    /// Set and send flow state
    async fn send_flow<W>(
        &mut self,
        writer: &mut W,
        link_credit: Option<u32>,
        drain: Option<bool>, // TODO: Is Option necessary?
        echo: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin;

    async fn on_incomplete_transfer(
        &mut self,
        delivery_tag: DeliveryTag,
        section_number: u32,
        section_offset: u64,
    );

    // More than one transfer frames should be hanlded by the
    // `Receiver`
    async fn on_incoming_transfer<T>(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<
        (
            Delivery<T>,
            Option<(DeliveryNumber, DeliveryTag, DeliveryState)>,
        ),
        Self::Error,
    >
    where
        T: for<'de> serde::Deserialize<'de> + Send;

    async fn dispose<W>(
        &mut self,
        writer: &mut W,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        // settled: bool, // TODO: This should depend on ReceiverSettleMode?
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::Error>
    where
        W: Sink<LinkFrame> + Send + Unpin;
}
