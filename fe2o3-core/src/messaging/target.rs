use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};
use fe2o3_amqp::primitives::{Boolean, Symbol};

use crate::definitions::Seconds;

use super::{Address, NodeProperties, TerminusDurability, TerminusExpiryPolicy};

/// 3.5.4 Target
///
/// <type name="target" class="composite" source="list" provides="target">
///     <descriptor name="amqp:target:list" code="0x00000000:0x00000029"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:target:list",
    code = 0x0000_0000_0000_0029,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Target {
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
    
    /// <field name="capabilities" type="symbol" multiple="true"/>
    capabilities: Vec<Symbol>,
}
