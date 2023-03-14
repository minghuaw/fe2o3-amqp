use serde::{Deserialize, Serialize};
use serde_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::{OrderedMap, UByte},
    value::Value,
};
use std::ops::{Deref, DerefMut};

use crate::primitives::SimpleValue;

pub mod map_builder;

pub mod annotations;
pub use annotations::Annotations;

pub mod header;
pub use header::Header;

/// relative message priority
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Eq, PartialOrd, Ord, Hash)]
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
///
/// <type name="delivery-annotations" class="restricted" source="annotations" provides="section">
///     <descriptor name="amqp:delivery-annotations:map" code="0x00000000:0x00000071"/>
/// </type>
///
/// The delivery-annotations section is used for delivery-specific non-standard properties at the
/// head of the message. Delivery annotations convey information from the sending peer to the
/// receiving peer. If the recipient does not understand the annotation it cannot be acted upon and
/// its effects (such as any implied propagation) cannot be acted upon. Annotations might be
/// specific to one implementation, or common to multiple implementations. The capabilities
/// negotiated on link attach and on the source and target SHOULD be used to establish which anno-
/// tations a peer supports. A registry of defined annotations and their meanings is maintained
/// [AMQPDELANN]. The symbolic key “rejected” is reserved for the use of communicating error
/// information regarding rejected messages. Any values associated with the “rejected” key MUST be
/// of type error.
///
/// If the delivery-annotations section is omitted, it is equivalent to a delivery-annotations
/// section containing an empty map of annotations.
#[derive(
    Debug, Clone, Default, DeserializeComposite, SerializeComposite, PartialEq, Eq, PartialOrd, Ord,
    Hash
)]
#[amqp_contract(
    name = "amqp:delivery-annotations:map",
    code = "0x0000_0000:0x0000_0071",
    encoding = "basic", // A simple wrapper over a map
)]
pub struct DeliveryAnnotations(pub Annotations);

impl DeliveryAnnotations {
    /// Creates a builder for [`DeliveryAnnotations`]
    pub fn builder() -> MapBuilder<OwnedKey, Value, Self> {
        MapBuilder::new()
    }
}

impl Deref for DeliveryAnnotations {
    type Target = Annotations;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DeliveryAnnotations {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<MapBuilder<OwnedKey, Value, DeliveryAnnotations>> for DeliveryAnnotations {
    fn from(builder: MapBuilder<OwnedKey, Value, DeliveryAnnotations>) -> Self {
        builder.build()
    }
}

impl From<MapBuilder<OwnedKey, Value, DeliveryAnnotations>> for Option<DeliveryAnnotations> {
    fn from(builder: MapBuilder<OwnedKey, Value, DeliveryAnnotations>) -> Self {
        Some(builder.build())
    }
}

/// 3.2.3 Message Annotations <type name="message-annotations" class="restricted"
/// source="annotations" provides="section"> <descriptor name="amqp:message-annotations:map"
///     code="0x00000000:0x00000072"/> </type>
///
/// The message-annotations section is used for properties of the message which are aimed at the
/// infrastructure and SHOULD be propagated across every delivery step. Message annotations convey
/// information about the message. Intermediaries MUST propagate the annotations unless the
/// annotations are explicitly augmented or modified (e.g., by the use of the modified outcome).
///
/// The capabilities negotiated on link attach and on the source and target can be used to establish
/// which annotations a peer understands; however, in a network of AMQP intermediaries it might
/// not be possible to know if every intermediary will understand the annotation. Note that for some
/// annotations it might not be necessary for the intermediary to understand their purpose, i.e.,
/// they could be used purely as an attribute which can be filtered on.
///
/// A registry of defined annotations and their meanings is maintained [AMQPMESSANN].
///
/// If the message-annotations section is omitted, it is equivalent to a message-annotations section
/// containing an empty map of annotations.
#[derive(
    Debug, Clone, Default, SerializeComposite, DeserializeComposite, PartialEq, Eq, PartialOrd, Ord,
    Hash
)]
#[amqp_contract(
    name = "amqp:message-annotations:map",
    code = "0x0000_0000:0x0000_0072",
    encoding = "basic"
)]
pub struct MessageAnnotations(pub Annotations);

impl MessageAnnotations {
    /// Creates a builder for [`MessageAnnotations`]
    pub fn builder() -> MapBuilder<OwnedKey, Value, Self> {
        MapBuilder::new()
    }
}

impl Deref for MessageAnnotations {
    type Target = Annotations;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MessageAnnotations {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<MapBuilder<OwnedKey, Value, MessageAnnotations>> for MessageAnnotations {
    fn from(builder: MapBuilder<OwnedKey, Value, MessageAnnotations>) -> Self {
        builder.build()
    }
}

impl From<MapBuilder<OwnedKey, Value, MessageAnnotations>> for Option<MessageAnnotations> {
    fn from(builder: MapBuilder<OwnedKey, Value, MessageAnnotations>) -> Self {
        Some(builder.build())
    }
}

pub mod properties;
pub use properties::Properties;

use self::{annotations::OwnedKey, map_builder::MapBuilder};

/// 3.2.5 Application Properties
/// <type name="application-properties" class="restricted" source="map" provides="section">
///     <descriptor name="amqp:application-properties:map" code="0x00000000:0x00000074"/>
/// </type>
#[derive(
    Debug, Clone, Default, SerializeComposite, DeserializeComposite, PartialEq, Eq, PartialOrd, Ord,
    Hash
)]
#[amqp_contract(
    name = "amqp:application-properties:map",
    code = "0x0000_0000:0x0000_0074",
    encoding = "basic"
)]
pub struct ApplicationProperties(pub OrderedMap<String, SimpleValue>);

impl ApplicationProperties {
    /// Creates a builder for ApplicationProperties
    pub fn builder() -> MapBuilder<String, SimpleValue, Self> {
        MapBuilder::new()
    }
}

impl Deref for ApplicationProperties {
    type Target = OrderedMap<String, SimpleValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ApplicationProperties {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<MapBuilder<String, SimpleValue, ApplicationProperties>> for ApplicationProperties {
    fn from(builder: MapBuilder<String, SimpleValue, ApplicationProperties>) -> Self {
        builder.build()
    }
}

impl From<MapBuilder<String, SimpleValue, ApplicationProperties>>
    for Option<ApplicationProperties>
{
    fn from(builder: MapBuilder<String, SimpleValue, ApplicationProperties>) -> Self {
        Some(builder.build())
    }
}

mod data;
pub use data::*;

mod amqp_sequence;
pub use amqp_sequence::*;

mod amqp_value;
pub use amqp_value::*;

/// 3.2.9 Footer
/// Transport footers for a message.
/// <type name="footer" class="restricted" source="annotations" provides="section">
///     <descriptor name="amqp:footer:map" code="0x00000000:0x00000078"/>
/// </type>
#[derive(
    Debug, Clone, Default, SerializeComposite, DeserializeComposite, PartialEq, Eq, PartialOrd, Ord,
    Hash
)]
#[amqp_contract(
    name = "amqp:footer:map",
    code = "0x0000_0000:0x0000_0078",
    encoding = "basic"
)]
pub struct Footer(pub Annotations);

impl Footer {
    /// Creates a builder for [`Footer`]
    pub fn builder() -> MapBuilder<OwnedKey, Value, Self> {
        MapBuilder::new()
    }
}

impl Deref for Footer {
    type Target = Annotations;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Footer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<MapBuilder<OwnedKey, Value, Footer>> for Footer {
    fn from(builder: MapBuilder<OwnedKey, Value, Footer>) -> Self {
        builder.build()
    }
}

impl From<MapBuilder<OwnedKey, Value, Footer>> for Option<Footer> {
    fn from(builder: MapBuilder<OwnedKey, Value, Footer>) -> Self {
        Some(builder.build())
    }
}

mod message_id;
pub use message_id::*;

/// 3.2.15 Address String
/// Address of a node.
/// <type name="address-string" class="restricted" source="string" provides="address"/>
pub type Address = String;

/// 3.2.16 CONSTANTS
pub const MESSAGE_FORMAT: u32 = 0; // FIXME: type of message format?

#[cfg(test)]
mod tests {
    use serde_amqp::{primitives::Binary, to_vec};

    use super::{AmqpSequence, Header, Priority};

    #[test]
    fn test_serialize_deserialize_header() {
        let header = Header {
            durable: true,
            priority: Priority(0),
            ttl: None,
            first_acquirer: false,
            delivery_count: 0,
        };
        let serialized = to_vec(&header).unwrap();
        println!("{:x?}", &serialized);
    }

    #[test]
    fn test_display_data() {
        let b = Binary::from(vec![1u8, 2]);
        println!("{}", b.len());
    }

    #[test]
    fn test_display_amqp_sequence() {
        let seq = AmqpSequence(vec![0, 1, 2, 3]);
        println!("{}", seq);
    }
}
