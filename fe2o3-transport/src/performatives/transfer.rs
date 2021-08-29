use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::Boolean,
};

use crate::{
    definitions::{DeliveryNumber, DeliveryTag, Handle, MessageFormat, ReceivervSettleMode},
    DeliveryState,
};

#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:transfer:list",
    code = 0x0000_0000_0000_0014,
    encoding = "list",
    rename_field = "kebab-case"
)]
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
    batchable: Option<Boolean>,
}
