use fe2o3_amqp::{
    macros::AmqpContract,
    types::{Array, Symbol, Uint, Ushort},
};
use serde::{Deserialize, Serialize};

use crate::definitions::{Fields, IetfLanguageTag, Milliseconds};

#[derive(Debug, Serialize, Deserialize, AmqpContract)]
#[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:open:list",
    code = 0x0000_0000_0000_0010,
    encoding = "list"
)]
pub struct Open {
    container_id: String,
    hostname: Option<String>,
    max_frame_size: Uint,
    channel_max: Ushort,
    idle_time_out: Milliseconds,
    outgoing_locales: IetfLanguageTag,
    incoming_locales: IetfLanguageTag,
    offered_capabilities: Array<Symbol>,
    desired_capabilities: Array<Symbol>,
    properties: Fields,
}
