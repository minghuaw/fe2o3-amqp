use serde_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::Boolean,
};

use crate::definitions::{Error, Handle};

/// 2.7.7 Detach
/// Detach the link endpoint from the session.
/// <type name="detach" class="composite" source="list" provides="frame">
///     <descriptor name="amqp:detach:list" code="0x00000000:0x00000016"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:detach:list",
    code = 0x0000_0000_0000_0016,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Detach {
    /// <field name="handle" type="handle" mandatory="true"/>
    pub handle: Handle,

    /// <field name="closed" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub closed: Boolean,

    /// <field name="error" type="error"/>
    pub error: Option<Error>,
}
