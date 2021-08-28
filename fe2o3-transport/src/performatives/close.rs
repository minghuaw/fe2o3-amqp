use serde::{Deserialize, Serialize};

use fe2o3_amqp::macros::Described;

use crate::definitions::Error;

#[derive(Debug, Deserialize, Serialize, Described)]
#[serde(rename_all = "kebab-case")]
#[amqp_contract(name="amqp:close:list", code=0x0000_0000_0000_0018, encoding="list")]
pub struct Close {
    error: Option<Error>
}
