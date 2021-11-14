use serde_amqp::macros::{DeserializeComposite, SerializeComposite};
use serde_amqp::primitives::{Boolean, Symbol};

use crate::definitions::{Fields, Seconds};

use super::{
    Address, DistributionMode, FilterSet, NodeProperties, Outcome, TerminusDurability,
    TerminusExpiryPolicy,
};

/// 3.5.3 Source
///
/// <type name="source" class="composite" source="list" provides="source">
///     <descriptor name="amqp:source:list" code="0x00000000:0x00000028"/>
/// </type>
#[derive(Debug, Clone, Default, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:source:list",
    code = 0x0000_0000_0000_0028,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Source {
    /// <field name="address" type="*" requires="address"/>
    pub address: Option<Address>,

    /// <field name="durable" type="terminus-durability" default="none"/>
    #[amqp_contract(default)]
    pub durable: TerminusDurability,

    /// <field name="expiry-policy" type="terminus-expiry-policy" default="session-end"/>
    #[amqp_contract(default)]
    pub expiry_policy: TerminusExpiryPolicy,

    /// <field name="timeout" type="seconds" default="0"/>
    #[amqp_contract(default)]
    pub timeout: Seconds,

    /// <field name="dynamic" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub dynamic: Boolean,

    /// <field name="dynamic-node-properties" type="node-properties"/>
    pub dynamic_node_properties: Option<NodeProperties>,

    /// <field name="distribution-mode" type="symbol" requires="distribution-mode"/>
    pub distribution_mode: Option<DistributionMode>,

    /// <field name="filter" type="filter-set"/>
    pub filter: Option<FilterSet>,

    /// <field name="default-outcome" type="*" requires="outcome"/>
    pub default_outcome: Option<Outcome>,

    /// <field name="outcomes" type="symbol" multiple="true"/>
    pub outcomes: Option<Vec<Symbol>>,

    /// <field name="capabilities" type="symbol" multiple="true"/>
    pub capabilities: Option<Vec<Symbol>>,
}

impl Source {
    pub fn builder() -> Builder {
        Builder::new()
    }
}

pub struct Builder {
    source: Source
}

impl Builder {
    pub fn new() -> Self {
        Self {
            source: Default::default()
        }
    }

    pub fn address(mut self, address: impl Into<Address>) -> Self {
        self.source.address = Some(address.into());
        self
    }

    pub fn durable(mut self, durability: TerminusDurability) -> Self {
        self.source.durable = durability;
        self
    }

    pub fn expiry_policy(mut self, policy: TerminusExpiryPolicy) -> Self {
        self.source.expiry_policy = policy;
        self
    }

    pub fn timeout(mut self, timeout: impl Into<Seconds>) -> Self {
        self.source.timeout = timeout.into();
        self
    }

    pub fn dynamic(mut self, dynamic: bool) -> Self {
        self.source.dynamic = dynamic;
        self
    }

    pub fn dynamic_node_properties(mut self, properties: impl Into<Fields>) -> Self {
        self.source.dynamic_node_properties = Some(properties.into());
        self
    }

    pub fn distribution_mode(mut self, mode: DistributionMode) -> Self {
        self.source.distribution_mode = Some(mode);
        self
    }

    pub fn filter(mut self, filter_set: impl Into<FilterSet>) -> Self {
        self.source.filter = Some(filter_set.into());
        self
    }

    pub fn default_outcome(mut self, outcome: Outcome) -> Self {
        self.source.default_outcome = Some(outcome.into());
        self
    }

    pub fn outcomes(mut self, outcomes: Vec<Symbol>) -> Self {
        self.source.outcomes = Some(outcomes);
        self
    }

    pub fn capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.source.capabilities = Some(capabilities);
        self
    }

    pub fn build(self) -> Source {
        self.source
    }
}

impl<T: Into<Address>> From<T> for Source {
    fn from(val: T) -> Self {
        Self::builder()
            .address(val.into())
            .build()
    }
}

#[cfg(test)]
mod tests {
    use serde_amqp::to_vec;

    use super::Source;

    #[test]
    fn test_serialize_source() {
        let source = Source::builder()
            // .address("q1")
            .build();
        let buf = to_vec(&source).unwrap();
        println!("{:#01x?}", buf);
        // println!("{:#01?}", buf);
    }
}