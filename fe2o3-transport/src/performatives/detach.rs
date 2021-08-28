use serde::{Deserialize, Serialize};

use fe2o3_amqp::{macros::Described, types::Boolean};

use crate::definitions::{Error, Handle};

#[derive(Debug, Deserialize, Serialize, Described)]
#[serde(rename_all = "kebab-case")]
#[amqp_contract(name="amqp:detach:list", code=0x0000_0000_0000_0016, encoding="list")]
pub struct Detach {
    handle: Handle,
    closed: Option<Boolean>,
    error: Option<Error>
}
