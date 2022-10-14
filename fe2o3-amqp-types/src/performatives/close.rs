use serde_amqp::macros::{DeserializeComposite, SerializeComposite};

use crate::definitions::Error;

/// <type name="close" class="composite" source="list" provides="frame">
/// <descriptor name="amqp:close:list" code="0x00000000:0x00000018"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:close:list",
    code = "0x0000_0000:0x0000_0018",
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Close {
    /// <field name="error" type="error"/>
    pub error: Option<Error>,
}

impl Close {
    /// Creates a new `Close` performative
    pub fn new(error: Option<Error>) -> Self {
        Self { error }
    }
}
