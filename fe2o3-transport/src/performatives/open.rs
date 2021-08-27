use fe2o3_amqp::{
    macros::Described,
    types::{Symbol, Uint, Ushort},
};
use serde::{Deserialize, Serialize};

use crate::definitions::{Fields, IetfLanguageTag, Milliseconds};

#[derive(Debug, Serialize, Deserialize, Described)]
#[serde(rename_all = "kebab-case")]
#[amqp_contract( name = "amqp:open:list", code = 0x0000_0000_0000_0010, encoding = "list")]
pub struct Open {
    pub container_id: String,
    pub hostname: Option<String>,
    pub max_frame_size: Option<Uint>,
    pub channel_max: Option<Ushort>,
    pub idle_time_out: Option<Milliseconds>,
    pub outgoing_locales: Option<Vec<IetfLanguageTag>>,
    pub incoming_locales: Option<Vec<IetfLanguageTag>>,
    pub offered_capabilities: Option<Vec<Symbol>>,
    pub desired_capabilities: Option<Vec<Symbol>>,
    pub properties: Option<Fields>,
}
