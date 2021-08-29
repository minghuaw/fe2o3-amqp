use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};

use crate::definitions::Error;

#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:close:list",
    code = 0x0000_0000_0000_0018,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Close {
    error: Option<Error>,
}
