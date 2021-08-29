use std::collections::BTreeMap;

use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::{Symbol, Ulong},
    value::Value,
};

use crate::{
    definitions::{
        DeliveryTag, Fields, Handle, ReceivervSettleMode, Role, SenderSettleMode, SequenceNo,
    },
    Source, Target,
};

#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:attach:list",
    code = 0x0000_0000_0000_0012,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Attach {
    pub name: String,
    pub handle: Handle,
    pub role: Role,
    pub snd_settle_mode: Option<SenderSettleMode>,
    pub rcv_settle_mode: Option<ReceivervSettleMode>,
    pub source: Option<Source>,
    pub target: Option<Target>,
    pub unsettled: Option<BTreeMap<DeliveryTag, Value>>,
    pub incomplete_unsettled: Option<bool>,
    pub initial_delivery_count: Option<SequenceNo>,
    pub max_message_size: Option<Ulong>,
    pub offered_capabilities: Option<Vec<Symbol>>,
    pub desired_capabilities: Option<Vec<Symbol>>,
    pub properties: Option<Fields>,
}
