
use fe2o3_amqp::{macros::{DeserializeDescribed, SerializeDescribed}, types::Boolean};

use crate::{DeliveryState, definitions::{DeliveryNumber, DeliveryTag, Handle, MessageFormat, ReceivervSettleMode}};

#[derive(Debug, DeserializeDescribed, SerializeDescribed)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(name="amqp:transfer:list", code=0x0000_0000_0000_0014, encoding="list")]
pub struct Transfer {
    handle: Handle,
    delivery_id: Option<DeliveryNumber>,
    delivery_tag: Option<DeliveryTag>,
    message_format: Option<MessageFormat>,
    settled: Option<Boolean>,
    more: Option<Boolean>,
    rcv_settle_mode: Option<ReceivervSettleMode>,
    state: Option<DeliveryState>,
    resume: Option<Boolean>,
    aborted: Option<Boolean>,
    batchable: Option<Boolean>
}