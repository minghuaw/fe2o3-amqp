use serde::{
    de::{self, SeqAccess, VariantAccess},
    ser::SerializeStruct,
    Serialize,
};
use serde_amqp::{
    descriptor::Descriptor,
    __constants::DESCRIBED_BASIC,
    primitives::{Binary, Symbol, Timestamp},
};

use crate::definitions::{Milliseconds, SequenceNo};

use super::{
    Address, AmqpSequence, AmqpValue, ApplicationProperties, Data, DeliveryAnnotations, Footer,
    Header, MessageAnnotations, MessageId, Priority, Properties,
};

#[derive(Debug, Clone)]
pub struct Message {
    pub header: Option<Header>,
    pub delivery_annotations: Option<DeliveryAnnotations>,
    pub message_annotations: Option<MessageAnnotations>,
    pub properties: Option<Properties>,
    pub application_properties: Option<ApplicationProperties>,
    pub body_section: BodySection,
    pub footer: Option<Footer>,
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct(DESCRIBED_BASIC, 7)?;
        if let Some(header) = &self.header {
            state.serialize_field("header", header)?;
        }
        if let Some(delivery_annotations) = &self.delivery_annotations {
            state.serialize_field("delivery_annotations", delivery_annotations)?;
        }
        if let Some(message_annotations) = &self.message_annotations {
            state.serialize_field("message_annotations", message_annotations)?;
        }
        if let Some(properties) = &self.properties {
            state.serialize_field("properties", properties)?;
        }
        if let Some(application_properties) = &self.application_properties {
            state.serialize_field("application_properties", application_properties)?
        }
        state.serialize_field("body_section", &self.body_section)?;
        if let Some(footer) = &self.footer {
            state.serialize_field("footer", footer)?;
        }
        state.end()
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Message;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct Message")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut header = None;
        let mut delivery_annotations = None;
        let mut message_annotations = None;
        let mut properties = None;
        let mut application_properties = None;
        let mut body_section = None;
        let mut footer = None;

        for _ in 0..7 {
            let descriptor = match seq.next_element()? {
                Some(val) => val,
                None => break
            };

            println!("{:?}", &descriptor);

            match descriptor {
                Descriptor::Code(code) => match code {
                    0x0000_0000_0000_0070 => header = deserialize_header(&mut seq)?,
                    0x0000_0000_0000_0071 => {
                        delivery_annotations = deserialize_delivery_annotations(&mut seq)?
                    }
                    0x0000_0000_0000_0072 => {
                        message_annotations = deserialize_message_annotations(&mut seq)?
                    }
                    0x0000_0000_0000_0073 => properties = deserialize_properties(&mut seq)?,
                    0x0000_0000_0000_0074 => {
                        application_properties = deserialize_application_properties(&mut seq)?
                    }
                    0x0000_0000_0000_0075 => {
                        body_section = Some(BodySection::Data(deserialize_data(&mut seq)?))
                    }
                    0x0000_0000_0000_0076 => {
                        body_section =
                            Some(BodySection::Sequence(deserialize_amqp_sequence(&mut seq)?))
                    }
                    0x0000_0000_0000_0077 => {
                        body_section = Some(BodySection::Value(deserialize_amqp_value(&mut seq)?))
                    }
                    0x0000_0000_0000_0078 => footer = deserialize_footer(&mut seq)?,
                    _ => return Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
                },
                Descriptor::Name(name) => match name.as_str() {
                    "amqp:header:list" => header = deserialize_header(&mut seq)?,
                    "amqp:delivery-annotations:map" => {
                        delivery_annotations = deserialize_delivery_annotations(&mut seq)?
                    }
                    "amqp:message-annotations:map" => {
                        message_annotations = deserialize_message_annotations(&mut seq)?
                    }
                    "amqp:properties:list" => properties = deserialize_properties(&mut seq)?,
                    "amqp:application-properties:map" => {
                        application_properties = deserialize_application_properties(&mut seq)?
                    }
                    "amqp:data:binary" => {
                        body_section = Some(BodySection::Data(deserialize_data(&mut seq)?))
                    }
                    "amqp:amqp-sequence:list" => {
                        body_section =
                            Some(BodySection::Sequence(deserialize_amqp_sequence(&mut seq)?))
                    }
                    "amqp:amqp-value:*" => {
                        body_section = Some(BodySection::Value(deserialize_amqp_value(&mut seq)?))
                    }
                    "amqp:footer:map" => footer = deserialize_footer(&mut seq)?,
                    _ => return Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
                },
            }
        }

        Ok(Message {
            header,
            delivery_annotations,
            message_annotations,
            properties,
            application_properties,
            body_section: body_section.ok_or_else(|| de::Error::custom("Expecting BodySection"))?,
            footer,
        })
    }
}

// #[inline]
// fn deserialize_descriptor<'de, A>(seq: &mut A) -> Result<Option<Descriptor>, A::Error>
// where
//     A: de::SeqAccess<'de>,
// {
//     seq.next_element()?
// }

#[inline]
fn deserialize_header<'de, A>(seq: &mut A) -> Result<Option<Header>, A::Error>
where
    A: de::SeqAccess<'de>,
{
    let durable: bool = match seq.next_element()? {
        Some(val) => val,
        None => Default::default(),
    };
    let priority: Priority = match seq.next_element()? {
        Some(val) => val,
        None => Default::default(),
    };
    let ttl: Option<Milliseconds> = seq.next_element()?;
    let first_acquirer: bool = match seq.next_element()? {
        Some(val) => val,
        None => Default::default(),
    };
    let delivery_count: u32 = match seq.next_element()? {
        Some(val) => val,
        None => Default::default(),
    };
    Ok(Some(Header {
        durable,
        priority,
        ttl,
        first_acquirer,
        delivery_count,
    }))
}

#[inline]
fn deserialize_delivery_annotations<'de, A>(
    seq: &mut A,
) -> Result<Option<DeliveryAnnotations>, A::Error>
where
    A: de::SeqAccess<'de>,
{
    let annotations = seq.next_element()?;
    Ok(annotations.map(|val| DeliveryAnnotations(val)))
}

#[inline]
fn deserialize_message_annotations<'de, A>(
    seq: &mut A,
) -> Result<Option<MessageAnnotations>, A::Error>
where
    A: de::SeqAccess<'de>,
{
    let annotations = seq.next_element()?;
    Ok(annotations.map(|val| MessageAnnotations(val)))
}

#[inline]
fn deserialize_properties<'de, A>(seq: &mut A) -> Result<Option<Properties>, A::Error>
where
    A: de::SeqAccess<'de>,
{
    let message_id: Option<MessageId> = seq.next_element()?;
    let user_id: Option<Binary> = seq.next_element()?;
    let to: Option<Address> = seq.next_element()?;
    let subject: Option<String> = seq.next_element()?;
    let reply_to: Option<Address> = seq.next_element()?;
    let correlation_id: Option<MessageId> = seq.next_element()?;
    let content_type: Option<Symbol> = seq.next_element()?;
    let content_encoding: Option<Symbol> = seq.next_element()?;
    let absolute_expiry_time: Option<Timestamp> = seq.next_element()?;
    let creation_time: Option<Timestamp> = seq.next_element()?;
    let group_id: Option<String> = seq.next_element()?;
    let group_sequence: Option<SequenceNo> = seq.next_element()?;
    let reply_to_groud_id: Option<String> = seq.next_element()?;

    Ok(Some(Properties {
        message_id,
        user_id,
        to,
        subject,
        reply_to,
        correlation_id,
        content_type,
        content_encoding,
        absolute_expiry_time,
        creation_time,
        group_id,
        group_sequence,
        reply_to_groud_id,
    }))
}

#[inline]
fn deserialize_application_properties<'de, A>(
    seq: &mut A,
) -> Result<Option<ApplicationProperties>, A::Error>
where
    A: SeqAccess<'de>,
{
    let value = seq.next_element()?;
    Ok(value.map(|val| ApplicationProperties(val)))
}

#[inline]
fn deserialize_data<'de, A>(seq: &mut A) -> Result<Data, A::Error>
where
    A: SeqAccess<'de>,
{
    match seq.next_element()? {
        Some(val) => Ok(Data(val)),
        None => Err(de::Error::custom("Expecting Data")),
    }
}

#[inline]
fn deserialize_amqp_sequence<'de, A>(seq: &mut A) -> Result<AmqpSequence, A::Error>
where
    A: SeqAccess<'de>,
{
    match seq.next_element()? {
        Some(val) => Ok(AmqpSequence(val)),
        None => Err(de::Error::custom("Expecting AmqpSequence")),
    }
}

#[inline]
fn deserialize_amqp_value<'de, A>(seq: &mut A) -> Result<AmqpValue, A::Error>
where
    A: SeqAccess<'de>,
{
    match seq.next_element()? {
        Some(val) => Ok(AmqpValue(val)),
        None => Err(de::Error::custom("Expecting AmqpSequence")),
    }
}

#[inline]
fn deserialize_footer<'de, A>(seq: &mut A) -> Result<Option<Footer>, A::Error>
where
    A: SeqAccess<'de>,
{
    let annotations = seq.next_element()?;
    Ok(annotations.map(|val| Footer(val)))
}

impl<'de> de::Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            DESCRIBED_BASIC,
            &[
                "header",
                "delivery_annotations",
                "message_annotations",
                "properties",
                "application_properties",
                "body_section",
                "footer",
            ],
            Visitor {},
        )
    }
}

impl<T> From<T> for Message
where
    T: Into<BodySection>,
{
    fn from(value: T) -> Self {
        Message {
            header: None,
            delivery_annotations: None,
            message_annotations: None,
            properties: None,
            application_properties: None,
            body_section: value.into(),
            footer: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct EmptyBody {}

#[derive(Debug, Default)]
pub struct Builder<T> {
    pub header: Option<Header>,
    pub delivery_annotations: Option<DeliveryAnnotations>,
    pub message_annotations: Option<MessageAnnotations>,
    pub properties: Option<Properties>,
    pub application_properties: Option<ApplicationProperties>,
    pub body_section: T,
    pub footer: Option<Footer>,
}

impl Builder<EmptyBody> {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<T> Builder<T> {
    pub fn body_section(self, body_section: impl Into<BodySection>) -> Builder<BodySection> {
        Builder::<BodySection> {
            header: self.header,
            delivery_annotations: self.delivery_annotations,
            message_annotations: self.message_annotations,
            properties: self.properties,
            application_properties: self.application_properties,
            body_section: body_section.into(),
            footer: self.footer,
        }
    }

    pub fn header(mut self, header: impl Into<Option<Header>>) -> Self {
        self.header = header.into();
        self
    }

    pub fn delivery_annotations(
        mut self,
        delivery_annotations: impl Into<Option<DeliveryAnnotations>>,
    ) -> Self {
        self.delivery_annotations = delivery_annotations.into();
        self
    }

    pub fn message_annotations(
        mut self,
        message_annotations: impl Into<Option<MessageAnnotations>>,
    ) -> Self {
        self.message_annotations = message_annotations.into();
        self
    }

    pub fn properties(mut self, properties: impl Into<Option<Properties>>) -> Self {
        self.properties = properties.into();
        self
    }

    pub fn application_properties(
        mut self,
        appplication_properties: impl Into<Option<ApplicationProperties>>,
    ) -> Self {
        self.application_properties = appplication_properties.into();
        self
    }

    pub fn footer(mut self, footer: impl Into<Option<Footer>>) -> Self {
        self.footer = footer.into();
        self
    }
}

impl Builder<BodySection> {
    pub fn build(self) -> Message {
        Message {
            header: self.header,
            delivery_annotations: self.delivery_annotations,
            message_annotations: self.message_annotations,
            properties: self.properties,
            application_properties: self.application_properties,
            body_section: self.body_section,
            footer: self.footer,
        }
    }
}

/// Only one section of Data and one section of AmqpSequence
/// is supported for now
#[derive(Debug, Clone)]
pub enum BodySection {
    Data(Data),
    Sequence(AmqpSequence),
    Value(AmqpValue),
}

impl<T: Into<AmqpValue>> From<T> for BodySection {
    fn from(value: T) -> Self {
        BodySection::Value(value.into())
    }
}

impl From<AmqpSequence> for BodySection {
    fn from(val: AmqpSequence) -> Self {
        Self::Sequence(val)
    }
}

impl From<Data> for BodySection {
    fn from(val: Data) -> Self {
        Self::Data(val)
    }
}

mod body_section {
    use super::*;

    impl Serialize for BodySection {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match self {
                BodySection::Data(data) => data.serialize(serializer),
                BodySection::Sequence(seq) => seq.serialize(serializer),
                BodySection::Value(val) => val.serialize(serializer),
            }
        }
    }

    struct FieldVisitor {}

    #[derive(Debug)]
    enum Field {
        Data,
        Sequence,
        Value,
    }

    impl<'de> de::Visitor<'de> for FieldVisitor {
        type Value = Field;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter
                .write_str("BodySection variant. One of Vec<Data>, Vec<AmqpSequence>, AmqpValue")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v {
                "amqp:data:binary" => Ok(Field::Data),
                "amqp:amqp-sequence:list" => Ok(Field::Sequence),
                "amqp:amqp-value:*" => Ok(Field::Value),
                _ => return Err(de::Error::custom("Invalid descriptor code")),
            }
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v {
                0x0000_0000_0000_0075 => Ok(Field::Data),
                0x0000_0000_0000_0076 => Ok(Field::Sequence),
                0x0000_0000_0000_0077 => Ok(Field::Value),
                _ => return Err(de::Error::custom("Invalid descriptor code")),
            }
        }
    }

    impl<'de> de::Deserialize<'de> for Field {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_ignored_any(FieldVisitor {})
        }
    }

    struct Visitor {}

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = BodySection;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("enum BodySection")
        }

        fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
        where
            A: de::EnumAccess<'de>,
        {
            let (val, variant) = data.variant()?;

            match val {
                Field::Data => {
                    let data = variant.newtype_variant()?;
                    Ok(BodySection::Data(data))
                }
                Field::Sequence => {
                    let sequence = variant.newtype_variant()?;
                    Ok(BodySection::Sequence(sequence))
                }
                Field::Value => {
                    let value = variant.newtype_variant()?;
                    Ok(BodySection::Value(value))
                }
            }
        }
    }

    impl<'de> de::Deserialize<'de> for BodySection {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_enum(
                serde_amqp::__constants::UNTAGGED_ENUM,
                &["Data", "Sequence", "Value"],
                Visitor {},
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use serde_amqp::{from_slice, to_vec, value::Value};
    use serde_bytes::ByteBuf;

    use crate::messaging::{message::BodySection, AmqpSequence, AmqpValue, Data, Header};

    use super::Message;

    #[test]
    fn test_serialize_deserialize_body() {
        let data = b"amqp".to_vec();
        let data = Data(ByteBuf::from(data));
        let body = BodySection::Data(data);
        let serialized = to_vec(&body).unwrap();
        println!("{:x?}", serialized);
        let field: BodySection = from_slice(&serialized).unwrap();
        println!("{:?}", field);

        let body = BodySection::Sequence(AmqpSequence(vec![Value::Bool(true)]));
        let serialized = to_vec(&body).unwrap();
        println!("{:x?}", serialized);
        let field: BodySection = from_slice(&serialized).unwrap();
        println!("{:?}", field);

        let body = BodySection::Value(AmqpValue(Value::Bool(true)));
        let serialized = to_vec(&body).unwrap();
        println!("{:x?}", serialized);
        let field: BodySection = from_slice(&serialized).unwrap();
        println!("{:?}", field);
    }

    #[test]
    fn test_serialize_deserialize_message() {
        let message = Message {
            header: Some(Header {
                durable: true,
                ..Default::default()
            }),
            delivery_annotations: None,
            message_annotations: None,
            properties: None,
            application_properties: None,
            body_section: BodySection::Value(AmqpValue(Value::Bool(true))),
            footer: None,
        };
        let serialized = to_vec(&message).unwrap();
        println!("{:x?}", serialized);
        let deserialized: Message = from_slice(&serialized).unwrap();
        println!("{:?}", deserialized);
    }
}
