use serde_amqp::described::Described;
use serde_amqp::macros::{DeserializeComposite, SerializeComposite};
use serde_amqp::primitives::{Array, Boolean, Symbol, OrderedMap};
use serde_amqp::Value;

use crate::definitions::{Fields, Seconds};

use super::{
    Address, DistributionMode, FilterSet, LifetimePolicy, NodeProperties, Outcome,
    SupportedDistModes, TerminusDurability, TerminusExpiryPolicy,
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
    ///
    /// Properties of the dynamically created node
    ///
    /// If the dynamic field is not set to true this field MUST be left unset.
    ///
    /// When set by the receiving link endpoint, this field contains the desired properties of the
    /// node the receiver wishes to be created. When set by the sending link endpoint this field
    /// contains the actual properties of the dynamically created node. See subsection 3.5.9 Node
    /// Properties for standard node properties. A registry of other commonly used node-properties
    /// and their meanings is maintained [AMQPNODEPROP].
    pub dynamic_node_properties: Option<NodeProperties>,

    /// <field name="distribution-mode" type="symbol" requires="distribution-mode"/>
    pub distribution_mode: Option<DistributionMode>,

    /// <field name="filter" type="filter-set"/>
    pub filter: Option<FilterSet>,

    /// <field name="default-outcome" type="*" requires="outcome"/>
    pub default_outcome: Option<Outcome>,

    /// <field name="outcomes" type="symbol" multiple="true"/>
    pub outcomes: Option<Array<Symbol>>,

    /// <field name="capabilities" type="symbol" multiple="true"/>
    pub capabilities: Option<Array<Symbol>>,
}

impl Source {
    /// Creates a [`Source`] builder
    pub fn builder() -> SourceBuilder {
        SourceBuilder::new()
    }
}

/// [`Source`] builder
#[derive(Debug, Clone)]
pub struct SourceBuilder {
    source: Source,
}

impl Default for SourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceBuilder {
    /// Creates a [`Source`] builder
    pub fn new() -> Self {
        Self {
            source: Default::default(),
        }
    }

    /// Set the "address" field
    pub fn address(mut self, address: impl Into<Address>) -> Self {
        self.source.address = Some(address.into());
        self
    }

    /// Set the "durability" field
    pub fn durable(mut self, durability: TerminusDurability) -> Self {
        self.source.durable = durability;
        self
    }

    /// Set the "expiry-policy" field
    pub fn expiry_policy(mut self, policy: TerminusExpiryPolicy) -> Self {
        self.source.expiry_policy = policy;
        self
    }

    /// Set the "timeout" field
    pub fn timeout(mut self, timeout: Seconds) -> Self {
        self.source.timeout = timeout;
        self
    }

    /// Set the "dynamic" field
    pub fn dynamic(mut self, dynamic: bool) -> Self {
        self.source.dynamic = dynamic;
        self
    }

    /// Set the "dynamic-node-properties" field
    ///
    /// If the dynamic field is not set to true this field MUST be left unset.
    ///
    /// When set by the receiving link endpoint, this field contains the desired properties of the
    /// node the receiver wishes to be created. When set by the sending link endpoint this field
    /// contains the actual properties of the dynamically created node. See subsection 3.5.9 Node
    /// Properties for standard node properties. A registry of other commonly used node-properties
    /// and their meanings is maintained [AMQPNODEPROP].
    ///
    /// # Example
    ///
    /// ```rust
    /// use fe2o3_amqp_types::messaging::{Source, LifetimePolicy, DeleteOnClose};
    ///
    /// let source = Source::builder()
    ///     .dynamic(true)
    ///     .dynamic_node_properties(LifetimePolicy::DeleteOnClose(DeleteOnClose{}))
    ///     .build();
    /// ```
    pub fn dynamic_node_properties(mut self, properties: impl Into<Fields>) -> Self {
        self.source.dynamic_node_properties = Some(properties.into());
        self
    }

    /// Add a "lifetime-policy" to the "dynamic-node-properties" field
    ///
    /// If the dynamic field is not set to true this field MUST be left unset.
    ///
    /// When set by the receiving link endpoint, this field contains the desired properties of the
    /// node the receiver wishes to be created. When set by the sending link endpoint this field
    /// contains the actual properties of the dynamically created node. See subsection 3.5.9 Node
    /// Properties for standard node properties. A registry of other commonly used node-properties
    /// and their meanings is maintained [AMQPNODEPROP].
    ///
    /// # Example
    ///
    /// ```rust
    /// use fe2o3_amqp_types::messaging::{Source, DeleteOnClose};
    ///
    /// let source = Source::builder()
    ///     .dynamic(true)
    ///     .add_lifetime_policy(DeleteOnClose{})
    ///     .build();
    /// ```
    pub fn add_lifetime_policy(mut self, policy: impl Into<LifetimePolicy>) -> Self {
        let policy: LifetimePolicy = policy.into();
        match &mut self.source.dynamic_node_properties {
            Some(map) => {
                map.insert(Symbol::from("lifetime-policy"), Value::from(policy));
            }
            None => {
                self.source.dynamic_node_properties = Some(policy.into());
            }
        };
        self
    }

    /// Add "supported-dist-modes" entry to the "dynamic-node-properties" field
    ///
    /// If the dynamic field is not set to true this field MUST be left unset.
    ///
    /// When set by the receiving link endpoint, this field contains the desired properties of the
    /// node the receiver wishes to be created. When set by the sending link endpoint this field
    /// contains the actual properties of the dynamically created node. See subsection 3.5.9 Node
    /// Properties for standard node properties. A registry of other commonly used node-properties
    /// and their meanings is maintained [AMQPNODEPROP].
    ///
    /// # Example
    ///
    /// ```rust
    /// use fe2o3_amqp_types::messaging::{Source, DistributionMode};
    ///
    /// let source = Source::builder()
    ///     .dynamic(true)
    ///     .add_supported_dist_modes(DistributionMode::Move)
    ///     .build();
    /// ```
    pub fn add_supported_dist_modes(mut self, modes: impl Into<SupportedDistModes>) -> Self {
        let modes: SupportedDistModes = modes.into();
        match &mut self.source.dynamic_node_properties {
            Some(map) => {
                map.insert(Symbol::from("supported-dist-modes"), Value::from(modes));
            }
            None => self.source.dynamic_node_properties = Some(modes.into()),
        };
        self
    }

    /// Set the "distribution-mode" field
    pub fn distribution_mode(mut self, mode: DistributionMode) -> Self {
        self.source.distribution_mode = Some(mode);
        self
    }

    /// Set the "filter" field
    pub fn filter(mut self, filter_set: FilterSet) -> Self {
        self.source.filter = Some(filter_set);
        self
    }

    /// Add an entryto the "filter" field
    pub fn add_to_filter(
        mut self,
        key: impl Into<Symbol>,
        value: impl Into<Option<Described<Value>>>,
    ) -> Self {
        self.source
            .filter
            .get_or_insert(OrderedMap::new())
            .insert(key.into(), value.into());
        self
    }

    /// Set the "default-outcome" field
    pub fn default_outcome(mut self, outcome: Outcome) -> Self {
        self.source.default_outcome = Some(outcome);
        self
    }

    /// Set the "outcomes" field
    pub fn outcomes(mut self, outcomes: impl Into<Array<Symbol>>) -> Self {
        self.source.outcomes = Some(outcomes.into());
        self
    }

    /// Set the "capabilities" field
    pub fn capabilities(mut self, capabilities: impl Into<Array<Symbol>>) -> Self {
        self.source.capabilities = Some(capabilities.into());
        self
    }

    /// Build the [`Source`]
    pub fn build(self) -> Source {
        self.source
    }
}

impl<T: Into<Address>> From<T> for Source {
    fn from(val: T) -> Self {
        Self::builder().address(val.into()).build()
    }
}
