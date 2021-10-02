use serde_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::{Boolean, UInt},
};

use crate::definitions::{Fields, Handle, SequenceNo, TransferNumber};

/// 2.7.4 Flow
/// Update link state.
/// <type name="flow" class="composite" source="list" provides="frame">
///     <descriptor name="amqp:flow:list" code="0x00000000:0x00000013"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:flow:list",
    code = 0x0000_0000_0000_0013,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Flow {
    /// <field name="next-incoming-id" type="transfer-number"/>
    pub next_incoming_id: Option<TransferNumber>,

    /// <field name="incoming-window" type="uint" mandatory="true"/>
    pub incoming_window: UInt,

    /// <field name="next-outgoing-id" type="transfer-number" mandatory="true"/>
    pub next_outgoing_id: TransferNumber,

    /// <field name="outgoing-window" type="uint" mandatory="true"/>
    pub outgoing_window: UInt,

    /// <field name="handle" type="handle"/>
    pub handle: Option<Handle>,

    /// <field name="delivery-count" type="sequence-no"/>
    pub delivery_count: Option<SequenceNo>,

    /// <field name="link-credit" type="uint"/>
    pub link_credit: Option<UInt>,

    ///     <field name="available" type="uint"/>
    pub available: Option<UInt>,

    /// <field name="drain" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub drain: Boolean,

    /// <field name="echo" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub echo: Boolean,

    /// <field name="properties" type="fields"/>
    pub properties: Option<Fields>,
}
