use serde_amqp::macros::{DeserializeComposite, SerializeComposite};
use serde_amqp::primitives::{Boolean, Symbol};

use crate::definitions::{Fields, Seconds};

use super::{Address, NodeProperties, TerminusDurability, TerminusExpiryPolicy};

/// 3.5.4 Target
///
/// <type name="target" class="composite" source="list" provides="target">
///     <descriptor name="amqp:target:list" code="0x00000000:0x00000029"/>
/// </type>
#[derive(Debug, Clone, Default, DeserializeComposite, SerializeComposite)]
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
    capabilities: Option<Vec<Symbol>>,
}

impl Target {
    /// Creates a Builder for Target
    pub fn builder() -> Builder {
        Builder::new()
    }
}

impl<T: Into<Address>> From<T> for Target {
    fn from(val: T) -> Self {
        // Self {
        //     address: Some(val.into()),
        //     ..Default::default()
        // }
        Self::builder().address(val.into()).build()
    }
}

/// [`Target`] builder
#[derive(Debug, Clone)]
pub struct Builder {
    /// The [`Target`] instance being built
    pub target: Target,
}

impl Builder {
    /// Creates a new [`Target`] builder
    pub fn new() -> Self {
        Self {
            target: Default::default(),
        }
    }

    /// Set the "address" field
    pub fn address(mut self, address: impl Into<Address>) -> Self {
        self.target.address = Some(address.into());
        self
    }

    /// Set the "durable" field
    pub fn durable(mut self, durability: TerminusDurability) -> Self {
        self.target.durable = durability;
        self
    }

    /// Set the "expiry-policy" field
    pub fn expiry_policy(mut self, policy: TerminusExpiryPolicy) -> Self {
        self.target.expiry_policy = policy;
        self
    }

    /// Set the "timeout" field
    pub fn timeout(mut self, timeout: impl Into<Seconds>) -> Self {
        self.target.timeout = timeout.into();
        self
    }

    /// Set the "dynamic" field
    pub fn dynamic(mut self, dynamic: bool) -> Self {
        self.target.dynamic = dynamic;
        self
    }

    /// Set the "dynamic-node-properties" field
    pub fn dynamic_node_properties(mut self, properties: impl Into<Fields>) -> Self {
        self.target.dynamic_node_properties = Some(properties.into());
        self
    }

    /// Set the "capabilities" field
    pub fn capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.target.capabilities = Some(capabilities);
        self
    }

    /// Build the [`Target`]
    pub fn build(self) -> Target {
        self.target
    }
}
