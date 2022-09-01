//! Define filters from AMQP Capabilities Registry: Filters
//! https://svn.apache.org/repos/asf/qpid/trunk/qpid/specs/apache-filters.xml#section-legacy-amqp

use fe2o3_amqp_types::primitives::{SimpleValue, Symbol};
use serde_amqp::{
    described::Described, descriptor::Descriptor, value::Value, DeserializeComposite,
    SerializeComposite,
};

pub use fe2o3_amqp_types::primitives::OrderedMap;

/// 1.1 Legacy Amqp Direct Binding
/// <type name="legacy-amqp-direct-binding" class="restricted" source="string" provides="filter">
///     <descriptor name="apache.org:legacy-amqp-direct-binding:string" code="0x0000468C:0x00000000"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "apache.org:legacy-amqp-direct-binding:string",
    code = 0x0000_468c_0000_0000,
    encoding = "basic"
)]
pub struct LegacyAmqpDirectBinding(pub String);

impl LegacyAmqpDirectBinding {
    /// Creates a new `LegacyAmqpDirectBinding`
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the descriptor code
    pub fn descriptor_code() -> u64 {
        0x0000_468c_0000_0000
    }

    /// Returns the descriptor name
    pub fn descriptor_name() -> Symbol {
        Symbol::from("apache.org:legacy-amqp-direct-binding:string")
    }
}

impl From<LegacyAmqpDirectBinding> for Described<String> {
    fn from(value: LegacyAmqpDirectBinding) -> Self {
        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0000),
            value: value.0,
        }
    }
}

impl From<LegacyAmqpDirectBinding> for Described<Value> {
    fn from(value: LegacyAmqpDirectBinding) -> Self {
        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0000),
            value: Value::String(value.0),
        }
    }
}

impl From<LegacyAmqpDirectBinding> for Option<Described<Value>> {
    fn from(value: LegacyAmqpDirectBinding) -> Self {
        Some(value.into())
    }
}

/// 1.2 Legacy Amqp Topic Binding
/// <type name="legacy-amqp-topic-binding" class="restricted" source="string" provides="filter">
///     <descriptor name="apache.org:legacy-amqp-topic-binding:string" code="0x0000468C:0x00000001"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "apache.org:legacy-amqp-topic-binding:string",
    code = 0x0000_468c_0000_0001,
    encoding = "basic"
)]
pub struct LegacyAmqpTopicBinding(pub String);

impl LegacyAmqpTopicBinding {
    /// Creates a new `LegacyAmqpTopicBinding`
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the descriptor code
    pub fn descriptor_code() -> u64 {
        0x0000_468c_0000_0001
    }

    /// Returns the descriptor name
    pub fn descriptor_name() -> Symbol {
        Symbol::from("apache.org:legacy-amqp-topic-binding:string")
    }
}

impl From<LegacyAmqpTopicBinding> for Described<String> {
    fn from(value: LegacyAmqpTopicBinding) -> Self {
        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0001),
            value: value.0,
        }
    }
}

impl From<LegacyAmqpTopicBinding> for Described<Value> {
    fn from(value: LegacyAmqpTopicBinding) -> Self {
        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0001),
            value: Value::String(value.0),
        }
    }
}

impl From<LegacyAmqpTopicBinding> for Option<Described<Value>> {
    fn from(value: LegacyAmqpTopicBinding) -> Self {
        Some(value.into())
    }
}

/// 1.3 Legacy Amqp Headers Binding
/// <type name="legacy-amqp-headers-binding" class="restricted" source="map" provides="filter">
///     <descriptor name="apache.org:legacy-amqp-headers-binding:map" code="0x0000468C:0x00000002"/>
/// </type>
///
/// The map has the same restriction as the application-properties section,
/// namely that the keys of this are restricted to be of type string
/// (which excludes the possibility of a null key) and the values are restricted
/// to be of simple types only, that is, excluding map, list, and array types.
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "apache.org:legacy-amqp-headers-binding:map",
    code = 0x0000_468c_0000_0002,
    encoding = "basic"
)]
pub struct LegacyAmqpHeadersBinding(pub OrderedMap<String, SimpleValue>);

impl LegacyAmqpHeadersBinding {
    /// Returns the descriptor code
    pub fn descriptor_code() -> u64 {
        0x0000_468c_0000_0002
    }

    /// Returns the descriptor name
    pub fn descriptor_name() -> Symbol {
        Symbol::from("apache.org:legacy-amqp-headers-binding:map")
    }
}

impl From<LegacyAmqpHeadersBinding> for Described<OrderedMap<String, SimpleValue>> {
    fn from(value: LegacyAmqpHeadersBinding) -> Self {
        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0002),
            value: value.0,
        }
    }
}

impl From<LegacyAmqpHeadersBinding> for Described<Value> {
    fn from(value: LegacyAmqpHeadersBinding) -> Self {
        let value = value
            .0
            .into_iter()
            .map(|(k, v)| (Value::String(k), Value::from(v)))
            .collect();

        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0002),
            value: Value::Map(value),
        }
    }
}

impl From<LegacyAmqpHeadersBinding> for Option<Described<Value>> {
    fn from(value: LegacyAmqpHeadersBinding) -> Self {
        Some(value.into())
    }
}

/// 2.1 No Local Filter
/// <type name="no-local-filter" class="composite" source="list" provides="filter">
///     <descriptor name="apache.org:no-local-filter:list" code="0x0000468C:0x00000003"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "apache.org:no-local-filter:list",
    code = 0x0000_468c_0000_0003,
    encoding = "basic"
)]
pub struct NoLocalFilter(pub Vec<Value>);

impl NoLocalFilter {
    /// Returns the descriptor code
    pub fn descriptor_code() -> u64 {
        0x0000_468c_0000_0003
    }

    /// Returns the descriptor name
    pub fn descriptor_name() -> Symbol {
        Symbol::from("apache.org:no-local-filter:list")
    }
}

impl From<NoLocalFilter> for Described<Vec<Value>> {
    fn from(value: NoLocalFilter) -> Self {
        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0003),
            value: value.0,
        }
    }
}

impl From<NoLocalFilter> for Described<Value> {
    fn from(value: NoLocalFilter) -> Self {
        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0003),
            value: Value::List(value.0),
        }
    }
}

impl From<NoLocalFilter> for Option<Described<Value>> {
    fn from(value: NoLocalFilter) -> Self {
        Some(value.into())
    }
}

/// 2.2 Selector Filter
/// <type name="selector-filter" class="restricted" source="string" provides="filter">
///     <descriptor name="apache.org:selector-filter:string" code="0x0000468C:0x00000004"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "apache.org:selector-filter:string",
    code = 0x0000_468c_0000_0004,
    encoding = "basic"
)]
pub struct SelectorFilter(pub String);

impl SelectorFilter {
    /// Creates a new `SelectorFilter`
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the descriptor code
    pub fn descriptor_code() -> u64 {
        0x0000_468c_0000_0004
    }

    /// Returns the descriptor name
    pub fn descriptor_name() -> Symbol {
        Symbol::from("apache.org:selector-filter:string")
    }
}

impl From<SelectorFilter> for Described<String> {
    fn from(value: SelectorFilter) -> Self {
        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0004),
            value: value.0,
        }
    }
}

impl From<SelectorFilter> for Described<Value> {
    fn from(value: SelectorFilter) -> Self {
        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0004),
            value: Value::String(value.0),
        }
    }
}

impl From<SelectorFilter> for Option<Described<Value>> {
    fn from(value: SelectorFilter) -> Self {
        Some(value.into())
    }
}

/// 3.1 Xquery
/// <type name="xquery" class="restricted" source="string" provides="filter">
///     <descriptor name="apache.org:xquery-filter:string" code="0x0000468C:0x00000005"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "apache.org:xquery-filter:string",
    code = 0x0000_468c_0000_0005,
    encoding = "basic"
)]
pub struct Xquery(pub String);

impl Xquery {
    /// Creates a new `Xquery`
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the descriptor code
    pub fn descriptor_code() -> u64 {
        0x0000_468c_0000_0005
    }

    /// Returns the descriptor name
    pub fn descriptor_name() -> Symbol {
        Symbol::from("apache.org:xquery-filter:string")
    }
}

impl From<Xquery> for Described<String> {
    fn from(value: Xquery) -> Self {
        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0005),
            value: value.0,
        }
    }
}

impl From<Xquery> for Described<Value> {
    fn from(value: Xquery) -> Self {
        Self {
            descriptor: Descriptor::Code(0x0000_468c_0000_0005),
            value: Value::String(value.0),
        }
    }
}

impl From<Xquery> for Option<Described<Value>> {
    fn from(value: Xquery) -> Self {
        Some(value.into())
    }
}
