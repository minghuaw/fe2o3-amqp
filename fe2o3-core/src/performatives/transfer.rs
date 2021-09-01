use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::Boolean,
};

use crate::{definitions::{DeliveryNumber, DeliveryTag, Handle, MessageFormat, ReceivervSettleMode}, messaging::DeliveryState};

#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:transfer:list",
    code = 0x0000_0000_0000_0014,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Transfer {
    pub handle: Handle,
    pub delivery_id: Option<DeliveryNumber>,
    pub delivery_tag: Option<DeliveryTag>,
    pub message_format: Option<MessageFormat>,
    pub settled: Option<Boolean>,
    pub more: Option<Boolean>,
    pub rcv_settle_mode: Option<ReceivervSettleMode>,
    pub state: Option<DeliveryState>,
    pub resume: Option<Boolean>,
    pub aborted: Option<Boolean>,
    pub batchable: Option<Boolean>,
}
