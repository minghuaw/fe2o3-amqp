use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::{Symbol, Uint, Ushort},
};

use crate::definitions::{Fields, Handle, TransferNumber};

/// 2.7.2 Begin
/// Begin a session on a channel.
/// <type name="begin" class="composite" source="list" provides="frame">
///     <descriptor name="amqp:begin:list" code="0x00000000:0x00000011"/>
///     ...
/// </type>
#[derive(Debug, SerializeComposite, DeserializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:begin:list",
    code = 0x0000_0000_0000_0011,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Begin {
    /// <field name="remote-channel" type="ushort"/>
    pub remote_channel: Option<Ushort>,

    /// <field name="next-outgoing-id" type="transfer-number" mandatory="true"/>
    pub next_outgoing_id: TransferNumber,

    /// <field name="incoming-window" type="uint" mandatory="true"/>
    pub incoming_window: Uint,

    /// <field name="outgoing-window" type="uint" mandatory="true"/>
    pub outgoing_window: Uint,

    /// <field name="handle-max" type="handle" default="4294967295"/>
    #[amqp_contract(default)]
    pub handle_max: Handle, // default to 4294967295

    /// <field name="offered-capabilities" type="symbol" multiple="true"/>
    pub offered_capabilities: Option<Vec<Symbol>>,

    /// <field name="desired-capabilities" type="symbol" multiple="true"/>
    pub desired_capabilities: Option<Vec<Symbol>>,

    /// <field name="properties" type="fields"/>
    pub properties: Option<Fields>,
}
