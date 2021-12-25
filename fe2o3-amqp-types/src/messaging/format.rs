use serde::{Deserialize, Serialize};
use serde_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::{Binary, Boolean, Symbol, Timestamp, UByte, UInt, ULong, Uuid},
    value::Value,
};
use std::collections::BTreeMap;

use crate::{
    definitions::{Milliseconds, SequenceNo},
    primitives::SimpleValue,
};

/// 3.2.1 Header
/// Transport headers for a message.
/// <type name="header" class="composite" source="list" provides="section">
///     <descriptor name="amqp:header:list" code="0x00000000:0x00000070"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:header:list",
    code = 0x0000_0000_0000_0070,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Header {
    /// <field name="durable" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub durable: Boolean,

    /// <field name="priority" type="ubyte" default="4"/>
    #[amqp_contract(default)]
    pub priority: Priority,

    /// <field name="ttl" type="milliseconds"/>
    pub ttl: Option<Milliseconds>,

    /// <field name="first-acquirer" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub first_acquirer: Boolean,

    /// <field name="delivery-count" type="uint" default="0"/>
    #[amqp_contract(default)]
    pub delivery_count: UInt,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Priority(pub UByte);

impl Default for Priority {
    fn default() -> Self {
        Self(4)
    }
}

impl From<UByte> for Priority {
    fn from(value: UByte) -> Self {
        Self(value)
    }
}

impl From<Priority> for UByte {
    fn from(value: Priority) -> Self {
        value.0
    }
}

/// 3.2.2 Delivery Annotations
/// <type name="delivery-annotations" class="restricted" source="annotations" provides="section">
///     <descriptor name="amqp:delivery-annotations:map" code="0x00000000:0x00000071"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:delivery-annotations:map",
    code = 0x0000_0000_0000_0071,
    encoding = "basic", // A simple wrapper over a map
)]
pub struct DeliveryAnnotations(pub Annotations);

/// 3.2.3 Message Annotations
/// <type name="message-annotations" class="restricted" source="annotations" provides="section">
///     <descriptor name="amqp:message-annotations:map" code="0x00000000:0x00000072"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:message-annotations:map",
    code = 0x0000_0000_0000_0072,
    encoding = "basic"
)]
pub struct MessageAnnotations(pub Annotations);

/// 3.2.4 Properties
/// Immutable properties of the message.
/// <type name="properties" class="composite" source="list" provides="section">
///     <descriptor name="amqp:properties:list" code="0x00000000:0x00000073"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:properties:list",
    code = 0x0000_0000_0000_0073,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Properties {
    /// <field name="message-id" type="*" requires="message-id"/>
    message_id: Option<MessageId>,

    /// <field name="user-id" type="binary"/>
    user_id: Option<Binary>,

    /// <field name="to" type="*" requires="address"/>
    to: Option<Address>,

    /// <field name="subject" type="string"/>
    subject: Option<String>,

    /// <field name="reply-to" type="*" requires="address"/>
    reply_to: Option<Address>,

    /// <field name="correlation-id" type="*" requires="message-id"/>
    correlation_id: Option<MessageId>,

    /// <field name="content-type" type="symbol"/>
    content_type: Option<Symbol>,

    /// <field name="content-encoding" type="symbol"/>
    content_encoding: Option<Symbol>,

    /// <field name="absolute-expiry-time" type="timestamp"/>
    absolute_expiry_time: Option<Timestamp>,

    /// <field name="creation-time" type="timestamp"/>
    creation_time: Option<Timestamp>,

    /// <field name="group-id" type="string"/>
    group_id: Option<String>,

    /// <field name="group-sequence" type="sequence-no"/>
    group_sequence: Option<SequenceNo>,

    /// <field name="reply-to-group-id" type="string"/>
    reply_to_groud_id: Option<String>,
}

/// 3.2.5 Application Properties
/// <type name="application-properties" class="restricted" source="map" provides="section">
///     <descriptor name="amqp:application-properties:map" code="0x00000000:0x00000074"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:application-properties:map",
    code = 0x0000_0000_0000_0074,
    encoding = "basic"
)]
pub struct ApplicationProperties(pub BTreeMap<String, SimpleValue>);

/// 3.2.6 Data
/// <type name="data" class="restricted" source="binary" provides="section">
///     <descriptor name="amqp:data:binary" code="0x00000000:0x00000075"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:data:binary",
    code = 0x0000_0000_0000_0075,
    encoding = "basic"
)]
pub struct Data(pub Binary);

impl TryFrom<Value> for Data {
    type Error = Value;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Binary(buf) = value {
            Ok(Data(buf))
        } else {
            Err(value)
        }
    }
}

/// 3.2.7 AMQP Sequence
/// <type name="amqp-sequence" class="restricted" source="list" provides="section">
///     <descriptor name="amqp:amqp-sequence:list" code="0x00000000:0x00000076"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:amqp-sequence:list",
    code = 0x0000_0000_0000_0076,
    encoding = "basic"
)]
pub struct AmqpSequence(pub Vec<Value>);

impl TryFrom<Value> for AmqpSequence {
    type Error = Value;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::List(vals) = value {
            Ok(AmqpSequence(vals))
        } else {
            Err(value)
        }
    }
}

/// 3.2.8 AMQP Value
/// <type name="amqp-value" class="restricted" source="*" provides="section">
///     <descriptor name="amqp:amqp-value:*" code="0x00000000:0x00000077"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:amqp-value:*",
    code = 0x0000_0000_0000_0077,
    encoding = "basic"
)]
pub struct AmqpValue(pub Value);

/// 3.2.9 Footer
/// Transport footers for a message.
/// <type name="footer" class="restricted" source="annotations" provides="section">
///     <descriptor name="amqp:footer:map" code="0x00000000:0x00000078"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:footer:map",
    code = 0x0000_0000_0000_0078,
    encoding = "basic"
)]
pub struct Footer(pub Annotations);

/// 3.2.10 Annotations
/// <type name="annotations" class="restricted" source="map"/>
pub type Annotations = BTreeMap<Symbol, Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageId {
    /// 3.2.11 Message ID ULong
    /// <type name="message-id-ulong" class="restricted" source="ulong" provides="message-id"/>
    ULong(ULong),

    /// 3.2.12 Message ID UUID
    /// <type name="message-id-uuid" class="restricted" source="uuid" provides="message-id"/>
    Uuid(Uuid),

    /// 3.2.13 Message ID Binary
    /// <type name="message-id-binary" class="restricted" source="binary" provides="message-id"/>
    Binary(Binary),

    /// 3.2.14 Message ID String
    /// <type name="message-id-string" class="restricted" source="string" provides="message-id"/>
    String(String),
}

/// 3.2.15 Address String
/// Address of a node.
/// <type name="address-string" class="restricted" source="string" provides="address"/>
pub type Address = String;

/// 3.2.16 CONSTANTS
pub const MESSAGE_FORMAT: u32 = 0; // FIXME: type of message format?

#[cfg(test)]
mod tests {
    
}