use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use fe2o3_amqp::{macros::Described, types::{Symbol, Ulong}, value::Value};

use crate::{Source, Target, definitions::{DeliveryTag, Fields, Handle, Role, SequenceNo, SndSettleMode}};

#[derive(Debug, Deserialize, Serialize, Described)]
#[serde(rename_all = "kebab-case")]
#[amqp_contract(name="amqp:attach:list", code=0x0000_0000_0000_0012, encoding="list")]
pub struct Attach {
    pub name: String,
    pub handle: Handle,
    pub role: Role,
    pub snd_settle_mode: Option<SndSettleMode>,
    pub rcv_settle_mode: Option<SndSettleMode>,
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
