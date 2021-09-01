use fe2o3_amqp::types::{Boolean, Symbol};
use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};

use crate::definitions::{Seconds};

/// 3.5.3 Source
/// 
/// <type name="source" class="composite" source="list" provides="source">
///     <descriptor name="amqp:source:list" code="0x00000000:0x00000028"/>
///     <field name="address" type="*" requires="address"/>
///     <field name="durable" type="terminus-durability" default="none"/>
///     <field name="expiry-policy" type="terminus-expiry-policy" default="session-end"/>
///     <field name="timeout" type="seconds" default="0"/>
///     <field name="dynamic" type="boolean" default="false"/>
///     <field name="dynamic-node-properties" type="node-properties"/>
///     <field name="distribution-mode" type="symbol" requires="distribution-mode"/>
///     <field name="filter" type="filter-set"/>
///     <field name="default-outcome" type="*" requires="outcome"/>
///     <field name="outcomes" type="symbol" multiple="true"/>
///     <field name="capabilities" type="symbol" multiple="true"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:source:list",
    code = 0x0000_0000_0000_0028,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Source {
    address: Option<Address>,
    durable: TerminusDurability, // TODO: implement default
    expiry_policy: TerminusExpiryPolicy,
    timeout: Seconds,
    dynamic: Boolean,
    dynamic_node_properties: Option<NodeProperties>,
    distribution_mode: Option<DistributionMode>,
    filter: Option<FilterSet>,
    default_outcome: Outcome,
    outcomes: Vec<Symbol>,
    capabilities: Vec<Symbol>
}   