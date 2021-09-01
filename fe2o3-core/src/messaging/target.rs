use fe2o3_amqp::types::{Boolean, Symbol};
use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};

use crate::definitions::{Seconds};

use super::{Address, NodeProperties, TerminusDurability, TerminusExpiryPolicy};

/// 3.5.4 Target
///
/// <type name="target" class="composite" source="list" provides="target">
///     <descriptor name="amqp:target:list" code="0x00000000:0x00000029"/>
///     <field name="address" type="*" requires="address"/>
///     <field name="durable" type="terminus-durability" default="none"/>
///     <field name="expiry-policy" type="terminus-expiry-policy" default="session-end"/>
///     <field name="timeout" type="seconds" default="0"/>
///     <field name="dynamic" type="boolean" default="false"/>
///     <field name="dynamic-node-properties" type="node-properties"/>
///     <field name="capabilities" type="symbol" multiple="true"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:target:list",
    code = 0x0000_0000_0000_0029,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Target {
    address: Option<Address>,
    durable: TerminusDurability, // TODO: impl default
    expiry_policy: TerminusExpiryPolicy,
    timeout: Seconds,
    dynamic: Boolean,
    dynamic_node_properties: Option<NodeProperties>,
    capabilities: Vec<Symbol>
}
