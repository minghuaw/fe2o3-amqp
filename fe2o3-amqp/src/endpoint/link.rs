//! Defines traits for link implementations

use std::future::Future;

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{
        DeliveryNumber, DeliveryTag, Error, Fields, MessageFormat, ReceiverSettleMode, Role,
        SequenceNo,
    },
    messaging::{DeliveryState, FromBody},
    performatives::{Attach, Detach, Transfer},
};
use tokio::sync::mpsc;

use crate::{
    control::SessionControl,
    link::{
        delivery::{Delivery, DeliveryInfo},
        state::LinkState,
        LinkFrame,
    },
    util::{AsByteIterator, IntoReader},
    Payload,
};

use super::{OutputHandle, Settlement};

pub(crate) trait LinkDetach {
    type DetachError: Send;

    fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::DetachError>;

    fn send_detach<'a>(
        &'a mut self,
        writer: &'a mpsc::Sender<LinkFrame>,
        closed: bool,
        error: Option<Error>,
    ) -> impl Future<Output = Result<(), Self::DetachError>> + Send + 'a;
}

pub(crate) trait LinkAttach {
    type AttachExchange: Send;
    type AttachError: Send;

    fn on_incoming_attach(
        &mut self,
        attach: Attach,
    ) -> Result<Self::AttachExchange, Self::AttachError>;

    fn send_attach<'a>(
        &'a mut self,
        writer: &'a mpsc::Sender<LinkFrame>,
        session: &'a mpsc::Sender<SessionControl>,
        is_reattaching: bool,
    ) -> impl Future<Output = Result<(), Self::AttachError>> + Send + 'a;
}

pub(crate) trait Link: LinkAttach + LinkDetach {
    fn role() -> Role;
}

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

    fn max_message_size(&self) -> Option<u64>;

    fn properties<F, O>(&self, op: F) -> O
    where
        F: FnOnce(&Option<Fields>) -> O;

    fn properties_mut<F, O>(&self, op: F) -> O
    where
        F: FnOnce(&mut Option<Fields>) -> O;

    fn exchange_attach<'a>(
        &'a mut self,
        writer: &'a mpsc::Sender<LinkFrame>,
        reader: &'a mut mpsc::Receiver<LinkFrame>,
        session: &'a mpsc::Sender<SessionControl>,
        is_reattaching: bool,
    ) -> impl Future<Output = Result<Self::AttachExchange, Self::AttachError>> + Send + 'a;

    fn handle_attach_error<'a>(
        &'a mut self,
        attach_error: Self::AttachError,
        writer: &'a mpsc::Sender<LinkFrame>,
        reader: &'a mut mpsc::Receiver<LinkFrame>,
        session: &'a mpsc::Sender<SessionControl>,
    ) -> impl Future<Output = Self::AttachError> + Send + 'a;
}

pub(crate) trait SenderLink: Link + LinkExt {
    type FlowError: Send;
    type TransferError: Send;
    type DispositionError: Send;

    /// Set and send flow state
    fn send_flow<'a>(
        &'a self,
        writer: &'a mpsc::Sender<LinkFrame>,
        delivery_count: Option<SequenceNo>,
        available: Option<u32>,
        echo: bool,
    ) -> impl Future<Output = Result<(), Self::FlowError>> + Send + 'a;

    /// Send message via transfer frame and return whether the message is already settled
    #[allow(clippy::too_many_arguments)]
    fn send_payload<'a, Fut>(
        &'a mut self,
        writer: &'a mpsc::Sender<LinkFrame>,
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
    ) -> impl Future<Output = Result<Settlement, Self::TransferError>> + Send + 'a
    where
        Fut: Future<Output = Option<LinkFrame>> + Send + 'a;

    /// Send message with delivery tag that is obtained by consuming a link credit
    fn send_payload_with_transfer<'a>(
        &'a mut self,
        writer: &'a mpsc::Sender<LinkFrame>,
        message_format: MessageFormat,
        transfer: Transfer,
        payload: Payload,
    ) -> impl Future<Output = Result<Settlement, Self::TransferError>> + Send + 'a;

    fn dispose<'a>(
        &'a mut self,
        writer: &'a mpsc::Sender<LinkFrame>,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> impl Future<Output = Result<(), Self::DispositionError>> + Send + 'a;

    fn batch_dispose<'a>(
        &'a mut self,
        writer: &'a mpsc::Sender<LinkFrame>,
        ids_and_tags: Vec<(DeliveryNumber, DeliveryTag)>,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> impl Future<Output = Result<(), Self::DispositionError>> + Send + 'a;
}

pub(crate) trait ReceiverLink: Link + LinkExt {
    type FlowError: Send;
    type TransferError: Send;
    type DispositionError: Send;

    /// Set and send flow state
    fn send_flow<'a>(
        &'a self,
        writer: &'a mpsc::Sender<LinkFrame>,
        link_credit: Option<u32>,
        drain: Option<bool>,
        echo: bool,
    ) -> impl Future<Output = Result<(), Self::FlowError>> + Send + 'a;

    /// Handles delivery state that is carried in a Transfer
    fn on_transfer_state(
        &mut self,
        delivery_tag: &Option<DeliveryTag>,
        settled: Option<bool>,
        state: DeliveryState,
    ) -> Result<(), Self::TransferError>;

    fn on_incomplete_transfer(
        &mut self,
        delivery_tag: DeliveryTag,
        section_number: u32,
        section_offset: u64,
    );

    // More than one transfer frames should be hanlded by the
    // `Receiver`
    fn on_complete_transfer<'a, T, P>(
        &'a mut self,
        transfer: Transfer,
        payload: P,
        section_number: u32,
        section_offset: u64,
    ) -> Result<Delivery<T>, Self::TransferError>
    where
        for<'de> T: FromBody<'de> + Send,
        for<'b> P: IntoReader + AsByteIterator<'b> + Send + 'a;

    fn dispose<'a>(
        &'a self,
        writer: &'a mpsc::Sender<LinkFrame>,
        delivery_info: DeliveryInfo,
        settled: Option<bool>,
        state: DeliveryState,
        batchable: bool,
    ) -> impl Future<Output = Result<(), Self::DispositionError>> + Send + 'a;

    fn dispose_all<'a>(
        &'a self,
        writer: &'a mpsc::Sender<LinkFrame>,
        delivery_infos: Vec<DeliveryInfo>,
        settled: Option<bool>,
        state: DeliveryState,
        batchable: bool,
    ) -> impl Future<Output = Result<(), Self::DispositionError>> + Send + 'a;
}
