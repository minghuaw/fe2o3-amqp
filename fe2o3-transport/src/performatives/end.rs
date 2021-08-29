use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};

use crate::definitions::Error;

// <type name="end" class="composite" source="list" provides="frame">
// <descriptor name="amqp:end:list" code="0x00000000:0x00000017"/>
// <field name="error" type="error"/>
// </type>

#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:end:list",
    code = 0x0000_0000_0000_0017,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct End {
    error: Option<Error>,
}
