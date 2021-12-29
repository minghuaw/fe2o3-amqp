use serde::{
    de::{self, VariantAccess},
    Deserialize, Serialize,
};
use serde_amqp::{
    described::Described, descriptor::Descriptor, format_code::EncodingCodes, value::Value,
};

use super::{
    AmqpSequence, AmqpValue, ApplicationProperties, Data, DeliveryAnnotations, Footer, Header,
    MessageAnnotations, Properties,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub header: Option<Header>,
    pub delivery_annotations: Option<DeliveryAnnotations>,
    pub message_annotations: Option<MessageAnnotations>,
    pub properties: Option<Properties>,
    pub application_properties: Option<ApplicationProperties>,
    pub body_section: BodySection,
    pub footer: Option<Footer>,
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

#[derive(Debug, Clone)]
pub enum BodySection {
    Data(Vec<Data>),
    Sequence(Vec<AmqpSequence>),
    Value(AmqpValue),
}

impl<T: Into<AmqpValue>> From<T> for BodySection {
    fn from(value: T) -> Self {
        BodySection::Value(value.into())
    }
}

// impl From<Vec<AmqpSequence>> for BodySection {
//     fn from(val: Vec<AmqpSequence>) -> Self {
//         Self::Sequence(val)
//     }
// }

// impl From<Vec<Data>> for BodySection {
//     fn from(val: Vec<Data>) -> Self {
//         Self::Data(val)
//     }
// }

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
    DataOrSequence,
    Value,
}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("BodySection variant. One of Vec<Data>, Vec<AmqpSequence>, AmqpValue")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v
            .try_into()
            .map_err(|_| de::Error::custom("Invalid format code for message body"))?
        {
            EncodingCodes::DescribedType => Field::Value,
            EncodingCodes::List0 | EncodingCodes::List8 | EncodingCodes::List32 => {
                Field::DataOrSequence
            }
            _ => return Err(de::Error::custom("Invalid format code for message body")),
        };
        Ok(val)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v {
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

        if let Field::Value = val {
            let value = variant.newtype_variant()?;
            Ok(BodySection::Value(value))
        } else {
            let values: Vec<Described<Value>> = variant.newtype_variant()?;

            let descriptor = values
                .first()
                .ok_or_else(|| de::Error::custom("Expecting either Data or AmqpSequence"))?
                .descriptor
                .as_ref();

            match descriptor {
                Descriptor::Code(code) => {
                    match code {
                        // Data
                        0x0000_0000_0000_0075 => {
                            let data: Result<Vec<Data>, _> = values
                                .into_iter()
                                .map(|d| Data::try_from(*d.value))
                                .collect();
                            data.map(|v| BodySection::Data(v))
                                .map_err(|_| de::Error::custom("Expecting Data"))
                        }
                        // Value
                        0x0000_0000_0000_0076 => {
                            let seq: Result<Vec<AmqpSequence>, _> = values
                                .into_iter()
                                .map(|d| AmqpSequence::try_from(*d.value))
                                .collect();
                            seq.map(|v| BodySection::Sequence(v))
                                .map_err(|_| de::Error::custom("Expecting AmqpSequence"))
                        }
                        _ => {
                            return Err(de::Error::custom("Expecting either Data or AmqpSequence"))
                        }
                    }
                }
                Descriptor::Name(name) => match name.as_str() {
                    "amqp:data:binary" => {
                        let data: Result<Vec<Data>, _> = values
                            .into_iter()
                            .map(|d| Data::try_from(*d.value))
                            .collect();
                        data.map(|v| BodySection::Data(v))
                            .map_err(|_| de::Error::custom("Expecting Data"))
                    }
                    "amqp:amqp-sequence:list" => {
                        let seq: Result<Vec<AmqpSequence>, _> = values
                            .into_iter()
                            .map(|d| AmqpSequence::try_from(*d.value))
                            .collect();
                        seq.map(|v| BodySection::Sequence(v))
                            .map_err(|_| de::Error::custom("Expecting AmqpSequence"))
                    }
                    _ => return Err(de::Error::custom("Expecting either Data or AmqpSequence")),
                },
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

#[cfg(test)]
mod tests {
    use std::vec;

    use serde_amqp::{from_slice, to_vec, value::Value};
    use serde_bytes::ByteBuf;

    use crate::messaging::{message::BodySection, AmqpSequence, Data};

    #[test]
    fn test_serialize_deserialize_body() {
        let data = b"amqp".to_vec();
        let data = vec![Data(ByteBuf::from(data))];
        let body = BodySection::Data(data);
        let serialized = to_vec(&body).unwrap();
        println!("{:x?}", serialized);
        let deserialized: BodySection = from_slice(&serialized).unwrap();
        println!("{:?}", deserialized);
    }

    #[test]
    fn test_field_deserializer() {
        // let data = b"amqp".to_vec();
        // let data = vec![Data(ByteBuf::from(data))];
        // let body = BodySection::Data(data);

        let body = BodySection::Sequence(vec![AmqpSequence(vec![Value::Bool(true)])]);

        // let body = BodySection::Value(AmqpValue(Value::Bool(true)));

        let serialized = to_vec(&body).unwrap();
        println!("{:x?}", serialized);
        let field: BodySection = from_slice(&serialized).unwrap();
        println!("{:?}", field);
    }
}
