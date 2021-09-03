use std::collections::BTreeMap;

use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::{Boolean, Symbol, Ulong},
    value::Value,
};

use crate::{
    definitions::{
        DeliveryTag, Fields, Handle, ReceiverSettleMode, Role, SenderSettleMode, SequenceNo,
    },
    messaging::{Source, Target},
};

/// 2.7.3 Attach
/// Attach a link to a session.
/// <type name="attach" class="composite" source="list" provides="frame">
///     <descriptor name="amqp:attach:list" code="0x00000000:0x00000012"/>
///     <field name="name" type="string" mandatory="true"/>
///     <field name="handle" type="handle" mandatory="true"/>
///     <field name="role" type="role" mandatory="true"/>
///     <field name="snd-settle-mode" type="sender-settle-mode" default="mixed"/>
///     <field name="rcv-settle-mode" type="receiver-settle-mode" default="first"/>
///     <field name="source" type="*" requires="source"/>
///     <field name="target" type="*" requires="target"/>
///     <field name="unsettled" type="map"/>
///     <field name="incomplete-unsettled" type="boolean" default="false"/>
///     <field name="initial-delivery-count" type="sequence-no"/>
///     <field name="max-message-size" type="ulong"/>
///     <field name="offered-capabilities" type="symbol" multiple="true"/>
///     <field name="desired-capabilities" type="symbol" multiple="true"/>
///     <field name="properties" type="fields"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:attach:list",
    code = 0x0000_0000_0000_0012,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Attach {
    /// <field name="name" type="string" mandatory="true"/>
    pub name: String,

    /// <field name="handle" type="handle" mandatory="true"/>
    pub handle: Handle,

    /// <field name="role" type="role" mandatory="true"/>
    pub role: Role,

    /// <field name="snd-settle-mode" type="sender-settle-mode" default="mixed"/>
    #[amqp_contract(default)]
    pub snd_settle_mode: SenderSettleMode,

    /// <field name="rcv-settle-mode" type="receiver-settle-mode" default="first"/>
    #[amqp_contract(default)]
    pub rcv_settle_mode: ReceiverSettleMode,

    /// <field name="source" type="*" requires="source"/>
    pub source: Option<Source>,

    /// <field name="target" type="*" requires="target"/>
    pub target: Option<Target>,

    /// <field name="unsettled" type="map"/>
    pub unsettled: Option<BTreeMap<DeliveryTag, Value>>,

    /// <field name="incomplete-unsettled" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub incomplete_unsettled: Boolean,

    /// <field name="initial-delivery-count" type="sequence-no"/>
    pub initial_delivery_count: Option<SequenceNo>,

    /// <field name="max-message-size" type="ulong"/>
    pub max_message_size: Option<Ulong>,

    /// <field name="offered-capabilities" type="symbol" multiple="true"/>
    pub offered_capabilities: Option<Vec<Symbol>>,

    /// <field name="desired-capabilities" type="symbol" multiple="true"/>
    pub desired_capabilities: Option<Vec<Symbol>>,

    /// <field name="properties" type="fields"/>
    pub properties: Option<Fields>,
}
