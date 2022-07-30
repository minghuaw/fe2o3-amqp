use fe2o3_amqp_types::{messaging::DeliveryState, definitions::{DeliveryNumber, DeliveryTag, ReceiverSettleMode}};


pub(crate) trait AsDeliveryState {
    fn as_delivery_state(&self) -> &Option<DeliveryState>;

    fn as_delivery_state_mut(&mut self) -> &mut Option<DeliveryState>;
}

impl AsDeliveryState for Option<DeliveryState> {
    fn as_delivery_state(&self) -> &Option<DeliveryState> {
        self
    }

    fn as_delivery_state_mut(&mut self) -> &mut Option<DeliveryState> {
        self
    }
}

pub(crate) struct DeliveryInfo {
    pub delivery_id: DeliveryNumber,
    pub delivery_tag: DeliveryTag,
    pub rcv_settle_mode: Option<ReceiverSettleMode>,
}