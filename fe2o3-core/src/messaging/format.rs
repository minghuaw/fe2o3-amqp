use std::collections::BTreeMap;

use fe2o3_amqp::{macros::{DeserializeComposite, SerializeComposite}, types::{Boolean, Symbol, Ubyte, Uint}, value::Value};

use crate::definitions::Milliseconds;

/// 3.2.1 Header
/// Transport headers for a message.
/// <type name="header" class="composite" source="list" provides="section">
///     <descriptor name="amqp:header:list" code="0x00000000:0x00000070"/>
///     <field name="durable" type="boolean" default="false"/>
///     <field name="priority" type="ubyte" default="4"/>
///     <field name="ttl" type="milliseconds"/>
///     <field name="first-acquirer" type="boolean" default="false"/>
///     <field name="delivery-count" type="uint" default="0"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:header:list",
    code = 0x0000_0000_0000_0070,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Header {
    durable: Boolean, // TODO: impl default to false
    priority: Ubyte, // TODO: impl default to 4
    ttl: Option<Milliseconds>,
    first_acquirer: Boolean, // TODO: impl default to false,
    delivery_count: Uint // TODO: impl default to 0
}


/// 3.2.2 Delivery Annotations
/// <type name="delivery-annotations" class="restricted" source="annotations" provides="section">
///     <descriptor name="amqp:delivery-annotations:map" code="0x00000000:0x00000071"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:delivery-annotations:map",
    code = 0x0000_0000_0000_0071,
)]
pub struct DeliveryAnnotations(BTreeMap<Symbol, Value>); // FIXME: how is newtype struct treated?