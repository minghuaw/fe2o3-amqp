use serde_amqp::macros::{DeserializeComposite, SerializeComposite};
use serde_amqp::primitives::{Boolean, Symbol};

use crate::definitions::Seconds;

use super::{
    Address, DistributionMode, FilterSet, NodeProperties, Outcome, TerminusDurability,
    TerminusExpiryPolicy,
};

/// 3.5.3 Source
///
/// <type name="source" class="composite" source="list" provides="source">
///     <descriptor name="amqp:source:list" code="0x00000000:0x00000028"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:source:list",
    code = 0x0000_0000_0000_0028,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Source {
    /// <field name="address" type="*" requires="address"/>
    address: Option<Address>,

    /// <field name="durable" type="terminus-durability" default="none"/>
    #[amqp_contract(default)]
    durable: TerminusDurability,

    /// <field name="expiry-policy" type="terminus-expiry-policy" default="session-end"/>
    #[amqp_contract(default)]
    expiry_policy: TerminusExpiryPolicy,

    /// <field name="timeout" type="seconds" default="0"/>
    #[amqp_contract(default)]
    timeout: Seconds,

    /// <field name="dynamic" type="boolean" default="false"/>
    #[amqp_contract(default)]
    dynamic: Boolean,

    /// <field name="dynamic-node-properties" type="node-properties"/>
    dynamic_node_properties: Option<NodeProperties>,

    /// <field name="distribution-mode" type="symbol" requires="distribution-mode"/>
    distribution_mode: Option<DistributionMode>,

    /// <field name="filter" type="filter-set"/>
    filter: Option<FilterSet>,

    /// <field name="default-outcome" type="*" requires="outcome"/>
    default_outcome: Outcome,

    /// <field name="outcomes" type="symbol" multiple="true"/>
    outcomes: Vec<Symbol>,

    /// <field name="capabilities" type="symbol" multiple="true"/>
    capabilities: Vec<Symbol>,
}
