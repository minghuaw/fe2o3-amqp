use serde_amqp::macros::{DeserializeComposite, SerializeComposite};
use serde_amqp::primitives::{Array, Boolean, Symbol};
use serde_amqp::Value;

use crate::definitions::{Fields, Seconds};

use super::{
    Address, LifetimePolicy, NodeProperties, SupportedDistModes, TerminusDurability,
    TerminusExpiryPolicy,
};

#[cfg(feature = "transaction")]
use crate::transaction::Coordinator;

/// The target archetype represented as a enum
///
/// For details, please see part 1.3, 3.5.4, and 4.5.1 in the core
/// specification.
#[derive(Debug, Clone)]
pub enum TargetArchetype {
    /// 3.5.4 Target
    Target(Target),

    /// 4.5.1 Coordinator
    #[cfg_attr(docsrs, doc(cfg(feature = "transaction")))]
    #[cfg(feature = "transaction")]
    Coordinator(Coordinator),
}

mod target_archetype_serde_impl {
    use serde::{
        de::{self, VariantAccess},
        ser,
    };

    use super::TargetArchetype;

    impl ser::Serialize for TargetArchetype {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match self {
                TargetArchetype::Target(value) => value.serialize(serializer),
                #[cfg(feature = "transaction")]
                TargetArchetype::Coordinator(value) => value.serialize(serializer),
            }
        }
    }

    enum Field {
        Target,
        #[cfg(feature = "transaction")]
        Coordinator,
    }

    struct FieldVisitor {}

    impl de::Visitor<'_> for FieldVisitor {
        type Value = Field;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("variant identifier for TargetArchetype")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let val = match v {
                "amqp:target:list" => Field::Target,
                #[cfg(feature = "transaction")]
                "amqp:coordinator:list" => Field::Coordinator,
                _ => {
                    return Err(de::Error::custom(
                        "Wrong descriptor symbol value for Target archetype",
                    ))
                }
            };
            Ok(val)
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let val = match v {
                0x0000_0000_0000_0029 => Field::Target,
                #[cfg(feature = "transaction")]
                0x0000_0000_0000_0030 => Field::Coordinator,
                _ => {
                    return Err(de::Error::custom(
                        "Wrong descriptor code value for Target archetype",
                    ))
                }
            };
            Ok(val)
        }
    }

    impl<'de> de::Deserialize<'de> for Field {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_identifier(FieldVisitor {})
        }
    }

    struct Visitor {}

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = TargetArchetype;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("variant identifier for TargetArchetype")
        }

        fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
        where
            A: de::EnumAccess<'de>,
        {
            let (val, variant) = data.variant()?;

            match val {
                Field::Target => {
                    let value = variant.newtype_variant()?;
                    Ok(TargetArchetype::Target(value))
                }
                #[cfg(feature = "transaction")]
                Field::Coordinator => {
                    let value = variant.newtype_variant()?;
                    Ok(TargetArchetype::Coordinator(value))
                }
            }
        }
    }

    impl<'de> de::Deserialize<'de> for TargetArchetype {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            #[cfg(not(feature = "transaction"))]
            const VARIANTS: &[&str] = &["amqp:target:list"];
            #[cfg(feature = "transaction")]
            const VARIANTS: &[&str] = &["amqp:target:list", "amqp:coordinator:list"];
            deserializer.deserialize_enum("TargetArchetype", VARIANTS, Visitor {})
        }
    }
}

/// 3.5.4 Target
///
/// <type name="target" class="composite" source="list" provides="target">
///     <descriptor name="amqp:target:list" code="0x00000000:0x00000029"/>
/// </type>
#[derive(Debug, Clone, Default, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:target:list",
    code = "0x0000_0000:0x0000_0029",
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Target {
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
    /// When set by the sending link endpoint, this field contains the desired properties of the
    /// node the sender wishes to be created. When set by the receiving link endpoint this field
    /// contains the actual properties of the dynamically created node. See subsection 3.5.9 Node
    /// Properties for standard node properties. A registry of other commonly used node-properties
    /// and their meanings is maintained [AMQPNODEPROP].
    pub dynamic_node_properties: Option<NodeProperties>,

    /// <field name="capabilities" type="symbol" multiple="true"/>
    pub capabilities: Option<Array<Symbol>>,
}

impl Target {
    /// Creates a TargetBuilder for Target
    pub fn builder() -> TargetBuilder {
        TargetBuilder::new()
    }
}

impl TryFrom<TargetArchetype> for Target {
    type Error = TargetArchetype;

    fn try_from(value: TargetArchetype) -> Result<Self, Self::Error> {
        match value {
            TargetArchetype::Target(target) => Ok(target),
            #[cfg(feature = "transaction")]
            _ => Err(value),
        }
    }
}

impl<T: Into<Address>> From<T> for Target {
    fn from(val: T) -> Self {
        Self {
            address: Some(val.into()),
            ..Default::default()
        }
        // Self::builder().address(val.into()).build()
    }
}

impl<T: Into<Target>> From<T> for TargetArchetype {
    fn from(value: T) -> Self {
        let target = value.into();
        Self::Target(target)
    }
}

/// [`Target`] builder
#[derive(Debug, Default, Clone)]
pub struct TargetBuilder {
    /// The [`Target`] instance being built
    pub target: Target,
}

impl TargetBuilder {
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
    pub fn timeout(mut self, timeout: Seconds) -> Self {
        self.target.timeout = timeout;
        self
    }

    /// Set the "dynamic" field
    pub fn dynamic(mut self, dynamic: bool) -> Self {
        self.target.dynamic = dynamic;
        self
    }

    /// Set the "dynamic-node-properties" field
    ///
    /// Properties of the dynamically created node
    ///
    /// If the dynamic field is not set to true this field MUST be left unset.
    ///
    /// When set by the sending link endpoint, this field contains the desired properties of the
    /// node the sender wishes to be created. When set by the receiving link endpoint this field
    /// contains the actual properties of the dynamically created node. See subsection 3.5.9 Node
    /// Properties for standard node properties. A registry of other commonly used node-properties
    /// and their meanings is maintained [AMQPNODEPROP].
    ///
    /// # Example
    ///
    /// ```rust
    /// use fe2o3_amqp_types::messaging::{Target, LifetimePolicy, DeleteOnClose};
    ///
    /// let source = Target::builder()
    ///     .dynamic(true)
    ///     .dynamic_node_properties(LifetimePolicy::DeleteOnClose(DeleteOnClose{}))
    ///     .build();
    /// ```
    pub fn dynamic_node_properties(mut self, properties: impl Into<Fields>) -> Self {
        self.target.dynamic_node_properties = Some(properties.into());
        self
    }

    /// Add a "lifetime-policy" to the "dynamic-node-properties" field
    ///
    /// If the dynamic field is not set to true this field MUST be left unset.
    ///
    /// When set by the sending link endpoint, this field contains the desired properties of the
    /// node the sender wishes to be created. When set by the receiving link endpoint this field
    /// contains the actual properties of the dynamically created node. See subsection 3.5.9 Node
    /// Properties for standard node properties. A registry of other commonly used node-properties
    /// and their meanings is maintained [AMQPNODEPROP].
    ///
    /// # Example
    ///
    /// ```rust
    /// use fe2o3_amqp_types::messaging::{Target, DeleteOnClose};
    ///
    /// let source = Target::builder()
    ///     .dynamic(true)
    ///     .add_lifetime_policy(DeleteOnClose{})
    ///     .build();
    /// ```
    pub fn add_lifetime_policy(mut self, policy: impl Into<LifetimePolicy>) -> Self {
        let policy: LifetimePolicy = policy.into();
        match &mut self.target.dynamic_node_properties {
            Some(map) => {
                map.insert(Symbol::from("lifetime-policy"), Value::from(policy));
            }
            None => {
                self.target.dynamic_node_properties = Some(policy.into());
            }
        };
        self
    }

    /// Add "supported-dist-modes" entry to the "dynamic-node-properties" field
    ///
    /// If the dynamic field is not set to true this field MUST be left unset.
    ///
    /// When set by the sending link endpoint, this field contains the desired properties of the
    /// node the sender wishes to be created. When set by the receiving link endpoint this field
    /// contains the actual properties of the dynamically created node. See subsection 3.5.9 Node
    /// Properties for standard node properties. A registry of other commonly used node-properties
    /// and their meanings is maintained [AMQPNODEPROP].
    ///
    /// # Example
    ///
    /// ```rust
    /// use fe2o3_amqp_types::messaging::{Target, DistributionMode};
    ///
    /// let source = Target::builder()
    ///     .dynamic(true)
    ///     .add_supported_dist_modes(DistributionMode::Move)
    ///     .build();
    /// ```
    pub fn add_supported_dist_modes(mut self, modes: impl Into<SupportedDistModes>) -> Self {
        let modes: SupportedDistModes = modes.into();
        match &mut self.target.dynamic_node_properties {
            Some(map) => {
                map.insert(Symbol::from("supported-dist-modes"), Value::from(modes));
            }
            None => self.target.dynamic_node_properties = Some(modes.into()),
        };
        self
    }

    /// Set the "capabilities" field
    pub fn capabilities(mut self, capabilities: impl Into<Array<Symbol>>) -> Self {
        self.target.capabilities = Some(capabilities.into());
        self
    }

    /// Build the [`Target`]
    pub fn build(self) -> Target {
        self.target
    }
}

#[cfg(test)]
mod tests {
    use serde_amqp::{from_slice, to_vec};

    use super::{Target, TargetArchetype};

    #[test]
    fn test_target_archetype_variant_target() {
        let target = Target::default();
        let buf = to_vec(&target).unwrap();
        let archetype: TargetArchetype = from_slice(&buf).unwrap();
        println!("{:?}", archetype);

        // println!("{:?}", std::mem::size_of::<Target>());
    }

    #[cfg(feature = "transaction")]
    #[test]
    fn test_target_archetype_variant_coordinator() {
        use crate::transaction::Coordinator;
        let coordinator = Coordinator::new(None);
        let buf = to_vec(&coordinator).unwrap();
        let archetype: TargetArchetype = from_slice(&buf).unwrap();
        println!("{:?}", archetype);

        // println!("{:?}", std::mem::size_of::<Coordinator>());
    }
}
