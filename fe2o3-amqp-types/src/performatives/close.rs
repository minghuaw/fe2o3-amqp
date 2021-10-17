use serde_amqp::macros::{DeserializeComposite, SerializeComposite};

use crate::definitions::Error;

#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:close:list",
    code = 0x0000_0000_0000_0018,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Close {
    pub error: Option<Error>,
}

impl Close {
    pub fn new(error: Option<Error>) -> Self {
        Self {
            error
        }
    }
}
