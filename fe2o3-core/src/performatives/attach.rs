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
    messaging::{Source, Target},
};

#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:attach:list",
    code = 0x0000_0000_0000_0012,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Attach {
    pub name: String,

    pub handle: Handle,
    
    pub role: Role,
    
    #[amqp_contract(default)]
    pub snd_settle_mode: SenderSettleMode,
    
    #[amqp_contract(default)]
    pub rcv_settle_mode: ReceivervSettleMode,
    
    pub source: Option<Source>,
    
    pub target: Option<Target>,
    
    pub unsettled: Option<BTreeMap<DeliveryTag, Value>>,
    
    #[amqp_contract(default)]
    pub incomplete_unsettled: bool,
    
    pub initial_delivery_count: Option<SequenceNo>,
    
    pub max_message_size: Option<Ulong>,
    
    pub offered_capabilities: Option<Vec<Symbol>>,
    
    pub desired_capabilities: Option<Vec<Symbol>>,
    
    pub properties: Option<Fields>,
}
