use fe2o3_amqp::{macros::{DeserializeComposite, SerializeComposite}, types::Boolean};

use crate::definitions::{Error, Handle};

#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(name="amqp:detach:list", code=0x0000_0000_0000_0016, encoding="list")]
pub struct Detach {
    handle: Handle,
    closed: Option<Boolean>,
    error: Option<Error>
}
