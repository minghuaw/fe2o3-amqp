use fe2o3_amqp::macros::{DeserializeDescribed, SerializeDescribed};

use crate::definitions::Error;

#[derive(Debug, DeserializeDescribed, SerializeDescribed)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(name="amqp:close:list", code=0x0000_0000_0000_0018, encoding="list")]
pub struct Close {
    error: Option<Error>
}
