//! Defines traits for link implementations

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{
        DeliveryNumber, DeliveryTag, Error, MessageFormat, ReceiverSettleMode, Role, SequenceNo,
    },
    messaging::DeliveryState,
    performatives::{Attach, Detach, Transfer},
};
use futures_util::Future;
use tokio::sync::mpsc;

use crate::{
    control::SessionControl,
    link::{delivery::Delivery, LinkFrame, state::LinkState},
    Payload,
};

use super::{OutputHandle, Settlement};

#[async_trait]
pub(crate) trait LinkDetach {
    type DetachError: Send;

    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::DetachError>;

    async fn send_detach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        closed: bool,
        error: Option<Error>,
    ) -> Result<(), Self::DetachError>;
}

#[async_trait]
pub(crate) trait LinkAttach {
    type AttachError: Send;

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::AttachError>;

    async fn send_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::AttachError>;
}

pub(crate) trait Link: LinkAttach + LinkDetach {
    fn role() -> Role;
}

#[async_trait]
pub(crate) trait LinkExt: Link {
    type FlowState;
    type Unsettled;
    type Target;

    fn local_state(&self) -> &LinkState;

    fn name(&self) -> &str;

    fn output_handle(&self) -> &Option<OutputHandle>;

    fn output_handle_mut(&mut self) -> &mut Option<OutputHandle>;

    fn flow_state(&self) -> &Self::FlowState;

    fn unsettled(&self) -> &Self::Unsettled;

    fn rcv_settle_mode(&self) -> &ReceiverSettleMode;

    fn target(&self) -> &Option<Self::Target>;

    async fn negotiate_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
    ) -> Result<(), Self::AttachError>;

    async fn handle_attach_error(
        &mut self,
        attach_error: Self::AttachError,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
        session: &mpsc::Sender<SessionControl>,
    ) -> Self::AttachError;
}

#[async_trait]
pub(crate) trait SenderLink: Link + LinkExt {
    type FlowError: Send;
    type TransferError: Send;
    type DispositionError: Send;

    /// Set and send flow state
    async fn send_flow(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        delivery_count: Option<SequenceNo>,
        available: Option<u32>,
        echo: bool,
    ) -> Result<(), Self::FlowError>;

    /// Send message via transfer frame and return whether the message is already settled
    async fn send_payload<Fut>(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
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
    ) -> Result<Settlement, Self::TransferError>
    where
        Fut: Future<Output = Option<LinkFrame>> + Send;

    async fn dispose(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError>;

    async fn batch_dispose(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        ids_and_tags: Vec<(DeliveryNumber, DeliveryTag)>,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError>;
}

#[async_trait]
pub(crate) trait ReceiverLink: Link + LinkExt {
    type FlowError: Send;
    type TransferError: Send;
    type DispositionError: Send;

    /// Set and send flow state
    async fn send_flow(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        link_credit: Option<u32>,
        drain: Option<bool>, // TODO: Is Option necessary?
        echo: bool,
    ) -> Result<(), Self::FlowError>;

    async fn on_incomplete_transfer(
        &mut self,
        delivery_tag: DeliveryTag,
        section_number: u32,
        section_offset: u64,
    );

    // More than one transfer frames should be hanlded by the
    // `Receiver`
    async fn on_complete_transfer<T>(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<
        (
            Delivery<T>,
            Option<(DeliveryNumber, DeliveryTag, DeliveryState)>,
        ),
        Self::TransferError,
    >
    where
        T: for<'de> serde::Deserialize<'de> + Send;

    async fn dispose(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        settled: Option<bool>, // TODO: This should depend on ReceiverSettleMode?
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError>;
}
