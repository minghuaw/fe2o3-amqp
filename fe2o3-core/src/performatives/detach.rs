use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::Boolean,
};

use crate::definitions::{Error, Handle};

#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:detach:list",
    code = 0x0000_0000_0000_0016,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Detach {
    pub handle: Handle,
    #[amqp_contract(default)]
    pub closed: Boolean,
    pub error: Option<Error>,
}
