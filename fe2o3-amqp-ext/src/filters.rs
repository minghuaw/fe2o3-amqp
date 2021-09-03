use std::collections::BTreeMap;

use fe2o3_amqp::{DeserializeComposite, SerializeComposite, value::Value};

// Define filters from AMQP Capabilities Registry: Filters
// https://svn.apache.org/repos/asf/qpid/trunk/qpid/specs/apache-filters.xml#section-legacy-amqp

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
pub struct LegacyAmqpHeadersBinding(pub BTreeMap<String, Value>);

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