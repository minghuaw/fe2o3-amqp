//! Defines traits for link implementations

use fe2o3_amqp_types::{
    definitions::{DeliveryNumber, DeliveryTag, Error, Fields, MessageFormat, ReceiverSettleMode},
    messaging::{DeliveryState, FromBody},
    performatives::{Attach, Detach, Transfer},
};
use futures_util::Future;
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

    async fn send_detach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        closed: bool,
        error: Option<Error>,
    ) -> Result<(), Self::DetachError>;
}

pub(crate) trait LinkAttach {
    type AttachExchange: Send;
    type AttachError: Send;

    fn on_incoming_attach(
        &mut self,
        attach: Attach,
    ) -> Result<Self::AttachExchange, Self::AttachError>;

    async fn send_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        session: &mpsc::Sender<SessionControl>,
        is_reattaching: bool,
    ) -> Result<(), Self::AttachError>;
}

pub(crate) trait Link: LinkAttach + LinkDetach {}

pub(crate) trait LinkExt: Link {
    type FlowState;
    type Unsettled;
    type Target;

    fn local_state(&self) -> &LinkState;

    fn name(&self) -> &str;

    fn output_handle_mut(&mut self) -> &mut Option<OutputHandle>;

    fn flow_state(&self) -> &Self::FlowState;

    fn unsettled(&self) -> &Self::Unsettled;

    fn rcv_settle_mode(&self) -> &ReceiverSettleMode;

    fn max_message_size(&self) -> Option<u64>;

    fn properties<F, O>(&self, op: F) -> O
    where
        F: FnOnce(&Option<Fields>) -> O;

    fn properties_mut<F, O>(&self, op: F) -> O
    where
        F: FnOnce(&mut Option<Fields>) -> O;

    async fn exchange_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
        session: &mpsc::Sender<SessionControl>,
        is_reattaching: bool,
    ) -> Result<Self::AttachExchange, Self::AttachError>;

    async fn handle_attach_error(
        &mut self,
        attach_error: Self::AttachError,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
        session: &mpsc::Sender<SessionControl>,
    ) -> Self::AttachError;
}

pub(crate) trait SenderLink: Link + LinkExt {
    type FlowError: Send;
    type TransferError: Send;
    type DispositionError: Send;

    /// Send message via transfer frame and return whether the message is already settled
    #[allow(clippy::too_many_arguments)]
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

    /// Send message with delivery tag that is obtained by consuming a link credit
    async fn send_payload_with_transfer(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        message_format: MessageFormat,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Settlement, Self::TransferError>;

    /// Note that it is possible for a disposition sent from sender to receiver
    /// to refer to a delivery which has not yet completed (i.e., a delivery
    /// which is spread over multiple frames and not all frames have yet been
    /// sent). The use of such interleaving is discouraged in favor of carrying
    /// the modified state on the next transfer performative for the delivery.
    #[allow(dead_code)]
    async fn dispose(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError>;

    /// Note that it is possible for a disposition sent from sender to receiver
    /// to refer to a delivery which has not yet completed (i.e., a delivery
    /// which is spread over multiple frames and not all frames have yet been
    /// sent). The use of such interleaving is discouraged in favor of carrying
    /// the modified state on the next transfer performative for the delivery.
    #[allow(dead_code)]
    async fn batch_dispose(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        ids_and_tags: Vec<(DeliveryNumber, DeliveryTag)>,
        settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError>;
}

pub(crate) trait ReceiverLink: Link + LinkExt {
    type FlowError: Send;
    type TransferError: Send;
    type DispositionError: Send;

    /// Set and send flow state
    async fn send_flow(
        &self,
        writer: &mpsc::Sender<LinkFrame>,
        link_credit: Option<u32>,
        drain: Option<bool>,
        echo: bool,
    ) -> Result<(), Self::FlowError>;

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

    async fn dispose(
        &self,
        writer: &mpsc::Sender<LinkFrame>,
        delivery_info: DeliveryInfo,
        settled: Option<bool>,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError>;

    async fn dispose_all(
        &self,
        writer: &mpsc::Sender<LinkFrame>,
        delivery_infos: Vec<DeliveryInfo>,
        settled: Option<bool>,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError>;
}
