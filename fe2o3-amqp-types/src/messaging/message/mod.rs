//! Implementation of Message as defined in AMQP 1.0 protocol Part 3.2

use std::{io, marker::PhantomData};

use serde::{
    de::{self},
    ser::SerializeStruct,
    Serialize,
};
use serde_amqp::__constants::{DESCRIBED_BASIC, DESCRIPTOR};

use super::{
    AmqpSequence, AmqpValue, ApplicationProperties, Batch, Data, DeliveryAnnotations, Footer,
    FromBody, Header, IntoBody, MessageAnnotations, Properties,
    SerializableBody,
};

mod body;
pub use body::*;

#[doc(hidden)]
pub mod __private {
    #[derive(Debug)]
    pub struct Serializable<T>(pub T);

    #[derive(Debug)]
    pub struct Deserializable<T>(pub T);
}
use __private::{Deserializable, Serializable};

/// Determines how a `Message<T>` should be docoded.
///
/// This is a byproduct of the workaround chosen for #49.
///
/// Why not `tokio_util::Decoder`
///
/// 1. avoid confusion
/// 2. The decoder type `T` itself is also the returned type
pub trait DecodeIntoMessage: Sized {
    /// Error type associated with decoding
    type DecodeError;

    /// Decode reader into [`Message<T>`]
    fn decode_into_message(reader: impl io::Read) -> Result<Message<Self>, Self::DecodeError>;
}

impl<T> DecodeIntoMessage for T
where
    for<'de> T: FromBody<'de>, // TODO: change to higher rank trait bound once GAT stablises
{
    type DecodeError = serde_amqp::Error;

    fn decode_into_message(reader: impl io::Read) -> Result<Message<Self>, Self::DecodeError> {
        let message: Deserializable<Message<T>> = serde_amqp::from_reader(reader)?;
        Ok(message.0)
    }
}

/// AMQP 1.0 Message
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Message<B> {
    /// Transport headers for a message.
    pub header: Option<Header>,

    /// The delivery-annotations section is used for delivery-specific non-standard properties at the head of the message.
    pub delivery_annotations: Option<DeliveryAnnotations>,

    /// The message-annotations section is used for properties of the message which are aimed at the infrastructure
    /// and SHOULD be propagated across every delivery step
    pub message_annotations: Option<MessageAnnotations>,

    /// Immutable properties of the message.
    pub properties: Option<Properties>,

    /// The application-properties section is a part of the bare message used for structured application data.
    /// Intermediaries can use the data within this structure for the purposes of filtering or routin
    pub application_properties: Option<ApplicationProperties>,

    /// The body consists of one of the following three choices: one or more data sections, one or more amqp-sequence
    /// sections, or a single amqp-value section.
    pub body: B,

    /// Transport footers for a message.
    pub footer: Option<Footer>,
}

impl<B> Serialize for Serializable<Message<B>>
where
    B: SerializableBody,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<B> Serialize for Serializable<&Message<B>>
where
    B: SerializableBody,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, B: FromBody<'de>> de::Deserialize<'de> for Deserializable<Message<B>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Message::<B>::deserialize(deserializer)?;
        Ok(Deserializable(value))
    }
}

impl Message<EmptyBody> {
    /// Creates a Builder for [`Message`]
    pub fn builder() -> Builder<EmptyBody> {
        Builder::new()
    }
}

impl<T> Message<T> {
    /// Count number of sections
    pub fn sections(&self) -> u32 {
        // The body section must be present
        let mut count = 1;

        if self.header.is_some() {
            count += 1;
        }
        if self.delivery_annotations.is_some() {
            count += 1;
        }
        if self.message_annotations.is_some() {
            count += 1;
        }
        if self.properties.is_some() {
            count += 1;
        }
        if self.application_properties.is_some() {
            count += 1;
        }
        if self.footer.is_some() {
            count += 1;
        }

        count
    }

    /// A complete message must have at least the body section, so we
    /// only need to whether footer is available
    pub fn last_section_code(&self) -> u8 {
        if self.footer.is_some() {
            0x78
        } else {
            0x77
        }
    }

    /// Map body to SerializableBody
    pub fn map_body<F, B>(self, op: F) -> Message<B>
    where
        F: FnOnce(T) -> B,
    {
        Message {
            header: self.header,
            delivery_annotations: self.delivery_annotations,
            message_annotations: self.message_annotations,
            properties: self.properties,
            application_properties: self.application_properties,
            body: (op)(self.body),
            footer: self.footer,
        }
    }
}

// impl<T> Serialize for Message<T>
impl<B> Message<B>
where
    B: SerializableBody,
{
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
        state.serialize_field("body", &self.body)?;
        if let Some(footer) = &self.footer {
            state.serialize_field("footer", footer)?;
        }
        state.end()
    }
}

enum Field {
    Header,
    DeliveryAnnotations,
    MessageAnnotations,
    Properties,
    ApplicationProperties,
    Body,
    Footer,
}

struct FieldVisitor {}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Field")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v {
            "amqp:header:list" => Field::Header,
            "amqp:delivery-annotations:map" => Field::DeliveryAnnotations,
            "amqp:message-annotations:map" => Field::MessageAnnotations,
            "amqp:properties:list" => Field::Properties,
            "amqp:application-properties:map" => Field::ApplicationProperties,
            "amqp:data:binary" | "amqp:amqp-sequence:list" | "amqp:amqp-value:*" => Field::Body,
            "amqp:footer:map" => Field::Footer,
            _ => return Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
        };
        Ok(val)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v {
            0x0000_0000_0000_0070 => Field::Header,
            0x0000_0000_0000_0071 => Field::DeliveryAnnotations,
            0x0000_0000_0000_0072 => Field::MessageAnnotations,
            0x0000_0000_0000_0073 => Field::Properties,
            0x0000_0000_0000_0074 => Field::ApplicationProperties,
            0x0000_0000_0000_0075 | 0x0000_0000_0000_0076 | 0x0000_0000_0000_0077 => Field::Body,
            0x0000_0000_0000_0078 => Field::Footer,
            _ => return Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
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

struct Visitor<B> {
    marker: PhantomData<B>,
}

impl<'de, B> de::Visitor<'de> for Visitor<B>
where
    B: FromBody<'de>,
{
    type Value = Message<B>;

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
        let mut body: Option<B> = None;
        let mut footer = None;

        let mut count = 0;
        while count < 7 {
            let field: Field = match seq.next_element()? {
                Some(val) => val,
                None => break,
            };

            #[allow(unused_variables)]
            match field {
                Field::Header => {
                    header = seq.next_element()?;
                    count += 1;
                }
                Field::DeliveryAnnotations => {
                    delivery_annotations = seq.next_element()?;
                    count += 1;
                }
                Field::MessageAnnotations => {
                    message_annotations = seq.next_element()?;
                    count += 1;
                }
                Field::Properties => {
                    properties = seq.next_element()?;
                    count += 1;
                }
                Field::ApplicationProperties => {
                    application_properties = seq.next_element()?;
                    count += 1;
                }
                Field::Body => {
                    let deserializable: Option<B::Body> = seq.next_element()?;
                    body =
                        deserializable.map(<B as FromBody>::from_body);
                    count += 1;
                }
                Field::Footer => {
                    footer = seq.next_element()?;
                    count += 1;
                }
            }
        }

        let body = match body {
            Some(body) => body,
            None => B::from_empty_body().map_err(|e| de::Error::custom(e))?,
        };

        Ok(Message {
            header,
            delivery_annotations,
            message_annotations,
            properties,
            application_properties,
            body,
            footer,
        })
    }
}

// impl<'de, T> de::Deserialize<'de> for Message<T>
impl<'de, B> Message<B>
where
    B: FromBody<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            DESCRIBED_BASIC,
            &[
                DESCRIPTOR,
                "header",
                DESCRIPTOR,
                "delivery_annotations",
                DESCRIPTOR,
                "message_annotations",
                DESCRIPTOR,
                "properties",
                DESCRIPTOR,
                "application_properties",
                DESCRIPTOR,
                "body",
                DESCRIPTOR,
                "footer",
            ],
            Visitor::<B> {
                marker: PhantomData,
            },
        )
    }
}

impl<T, U> From<T> for Message<U>
where
    T: IntoBody<Body = U>,
{
    fn from(value: T) -> Self {
        Message {
            header: None,
            delivery_annotations: None,
            message_annotations: None,
            properties: None,
            application_properties: None,
            body: value.into_body(),
            footer: None,
        }
    }
}

/// Convert from message
pub trait FromMessage<B> {
    /// Convert from message
    fn from_message(message: Message<B>) -> Self;
}

impl<T, B> FromMessage<B> for T
where
    for<'de> T: FromBody<'de, Body = B>,
{
    fn from_message(message: Message<B>) -> Self {
        Self::from_body(message.body)
    }
}

/// A type state representing undefined body for Message Builder
#[derive(Debug, Default)]
pub struct EmptyBody {}

/// [`Message`] builder
#[derive(Debug, Clone, Default)]
pub struct Builder<T> {
    /// header
    pub header: Option<Header>,
    /// delivery annotations
    pub delivery_annotations: Option<DeliveryAnnotations>,
    /// message annotations
    pub message_annotations: Option<MessageAnnotations>,
    /// properties
    pub properties: Option<Properties>,
    /// application properties
    pub application_properties: Option<ApplicationProperties>,
    /// body sections
    pub body: T,
    /// footer
    pub footer: Option<Footer>,
}

impl Builder<EmptyBody> {
    /// Creates a new [`Message`] builder
    pub fn new() -> Self {
        Default::default()
    }
}

impl<T> Builder<T> {
    /// Set the header
    pub fn header(mut self, header: impl Into<Option<Header>>) -> Self {
        self.header = header.into();
        self
    }

    /// Set the delivery annotations
    pub fn delivery_annotations(
        mut self,
        delivery_annotations: impl Into<Option<DeliveryAnnotations>>,
    ) -> Self {
        self.delivery_annotations = delivery_annotations.into();
        self
    }

    /// Set the message annotations
    pub fn message_annotations(
        mut self,
        message_annotations: impl Into<Option<MessageAnnotations>>,
    ) -> Self {
        self.message_annotations = message_annotations.into();
        self
    }

    /// Set properties
    pub fn properties(mut self, properties: impl Into<Option<Properties>>) -> Self {
        self.properties = properties.into();
        self
    }

    /// Set application properties
    pub fn application_properties(
        mut self,
        application_properties: impl Into<Option<ApplicationProperties>>,
    ) -> Self {
        self.application_properties = application_properties.into();
        self
    }

    /// Set footer
    pub fn footer(mut self, footer: impl Into<Option<Footer>>) -> Self {
        self.footer = footer.into();
        self
    }

    /// Set the body as `Body`
    pub fn body<B>(self, value: B) -> Builder<B> {
        Builder {
            header: self.header,
            delivery_annotations: self.delivery_annotations,
            message_annotations: self.message_annotations,
            properties: self.properties,
            application_properties: self.application_properties,
            body: value,
            footer: self.footer,
        }
    }

    /// Set the body as `Body::Value`
    pub fn value<V: Serialize>(self, value: V) -> Builder<AmqpValue<V>> {
        Builder {
            header: self.header,
            delivery_annotations: self.delivery_annotations,
            message_annotations: self.message_annotations,
            properties: self.properties,
            application_properties: self.application_properties,
            body: AmqpValue(value),
            footer: self.footer,
        }
    }

    /// Set the body as a single `Body::Sequence` section
    pub fn sequence<V: Serialize>(self, values: impl Into<Vec<V>>) -> Builder<AmqpSequence<V>> {
        Builder {
            header: self.header,
            delivery_annotations: self.delivery_annotations,
            message_annotations: self.message_annotations,
            properties: self.properties,
            application_properties: self.application_properties,
            body: AmqpSequence(values.into()),
            footer: self.footer,
        }
    }

    /// Set the body as `Body::SequenceBatch`
    pub fn sequence_batch<V: Serialize>(
        self,
        batch: impl Into<Batch<AmqpSequence<V>>>,
    ) -> Builder<Batch<AmqpSequence<V>>> {
        Builder {
            header: self.header,
            delivery_annotations: self.delivery_annotations,
            message_annotations: self.message_annotations,
            properties: self.properties,
            application_properties: self.application_properties,
            body: batch.into(),
            footer: self.footer,
        }
    }

    /// Set the body as a single `Body::Data` section
    pub fn data(self, data: impl Into<Data>) -> Builder<Data> {
        Builder {
            header: self.header,
            delivery_annotations: self.delivery_annotations,
            message_annotations: self.message_annotations,
            properties: self.properties,
            application_properties: self.application_properties,
            body: data.into(),
            footer: self.footer,
        }
    }

    /// Set the body as `Body::DataBatch`
    pub fn data_batch(self, batch: impl Into<Batch<Data>>) -> Builder<Batch<Data>> {
        Builder {
            header: self.header,
            delivery_annotations: self.delivery_annotations,
            message_annotations: self.message_annotations,
            properties: self.properties,
            application_properties: self.application_properties,
            body: batch.into(),
            footer: self.footer,
        }
    }

    /// Build the [`Message`]
    pub fn build(self) -> Message<T> {
        Message {
            header: self.header,
            delivery_annotations: self.delivery_annotations,
            message_annotations: self.message_annotations,
            properties: self.properties,
            application_properties: self.application_properties,
            body: self.body,
            footer: self.footer,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use serde_amqp::{from_reader, from_slice, primitives::Binary, to_vec, value::Value};
    use serde_bytes::ByteBuf;

    use crate::messaging::{
        message::{
            Body,
            __private::{Deserializable, Serializable},
        },
        AmqpSequence, AmqpValue, ApplicationProperties, Batch, Data, DeliveryAnnotations, Header,
        MessageAnnotations, Properties,
    };

    use super::Message;

    // NOTE: Messave<Value> mem size is only 8 bytes larger than Message<()>
    // fn print_message_size() {
    //     println!("Value: {:?}", std::mem::size_of::<Value>());
    //     println!("Message<Value>: {:?}", std::mem::size_of::<Message<Value>>());
    //     println!("Message<()>: {:?}", std::mem::size_of::<Message<()>>());
    // }

    #[test]
    fn test_convert_data_into_message() {
        let data = Data(Binary::from("hello AMQP"));
        let message = Message::from(data);
        let buf = to_vec(&Serializable(message)).unwrap();
        assert_eq!(buf[2], 0x75);
    }

    #[test]
    fn test_convert_amqp_sequence_into_message() {
        let sequence = AmqpSequence(vec![1, 2, 3, 4]);
        let message = Message::from(sequence);
        let buf = to_vec(&Serializable(message)).unwrap();
        assert_eq!(buf[2], 0x76);
    }

    #[test]
    fn test_convert_amqp_value_into_message() {
        let value = AmqpValue(vec![1, 2, 3, 4]);
        let message = Message::from(value);
        let buf = to_vec(&Serializable(message)).unwrap();
        assert_eq!(buf[2], 0x77);
    }

    #[test]
    fn test_serialize_deserialize_null() {
        let body = AmqpValue(Value::Null);
        let buf = to_vec(&body).unwrap();
        println!("{:#x?}", buf);

        let body2: AmqpValue<Value> = from_slice(&buf).unwrap();
        println!("{:?}", body2.0)
    }

    #[test]
    fn test_serialize_amqp_value_f32() {
        let message = Message::from(AmqpValue(3.14f32));
        let buf = to_vec(&Serializable(message)).unwrap();
        let expected = Message::from(Body::Value(AmqpValue(Value::from(3.14f32))));
        let expected = to_vec(&Serializable(expected)).unwrap();
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_serialize_deserialize_body() {
        let data = b"amqp".to_vec();
        let data = Data(ByteBuf::from(data));
        let body = Body::<Value>::Data(vec![data].into());
        let serialized = to_vec(&body).unwrap();
        println!("{:x?}", serialized);
        let field: Body<Value> = from_slice(&serialized).unwrap();
        println!("{:?}", field);

        let body = Body::Sequence(vec![AmqpSequence(vec![Value::Bool(true)])].into());
        let serialized = to_vec(&body).unwrap();
        println!("{:x?}", serialized);
        let field: Body<Value> = from_slice(&serialized).unwrap();
        println!("{:?}", field);

        let body = Body::Value(AmqpValue(Value::Bool(true)));
        let serialized = to_vec(&body).unwrap();
        println!("{:x?}", serialized);
        let field: Body<Value> = from_slice(&serialized).unwrap();
        println!("{:?}", field);
    }

    #[test]
    fn test_serialize_message() {
        let message = Message {
            header: Some(Header {
                durable: true,
                ..Default::default()
            }),
            // header: None,
            delivery_annotations: None,
            message_annotations: None,
            properties: None,
            application_properties: None,
            body: Body::Value(AmqpValue(Value::Bool(true))),
            footer: None,
        };
        let mut buf = Vec::new();
        let mut serializer = serde_amqp::ser::Serializer::new(&mut buf);
        message.serialize(&mut serializer).unwrap();
        println!("{:x?}", buf);
    }

    #[test]
    fn test_serialize_deserialize_message() {
        let message = Message {
            header: Some(Header {
                durable: true,
                ..Default::default()
            }),
            delivery_annotations: Some(DeliveryAnnotations::builder().insert("key", 1u32).build()),
            message_annotations: Some(MessageAnnotations::builder().insert("key2", "v").build()),
            // message_annotations: None,
            properties: Some(Properties::builder().message_id(1u64).build()),
            application_properties: Some(
                ApplicationProperties::builder().insert("sn", 1i32).build(),
            ),
            body: Body::Value(AmqpValue(Value::Bool(true))),
            footer: None,
        };
        let mut buf = Vec::new();
        let mut serializer = serde_amqp::ser::Serializer::new(&mut buf);
        message.serialize(&mut serializer).unwrap();
        let deserialized: Deserializable<Message<Body<Value>>> = from_slice(&buf).unwrap();

        assert!(deserialized.0.header.is_some());
        assert!(deserialized.0.delivery_annotations.is_some());
        assert!(deserialized.0.message_annotations.is_some());
        assert!(deserialized.0.properties.is_some());
        assert!(deserialized.0.application_properties.is_some());
        assert_eq!(
            deserialized.0.body,
            Body::Value(AmqpValue(Value::Bool(true)))
        );
        assert!(deserialized.0.footer.is_none());
    }

    #[test]
    fn test_decoding_message_with_no_body_section_from_slice() {
        let buf: [u8; 8] = [0x0, 0x53, 0x70, 0x45, 0x0, 0x53, 0x73, 0x45];
        let result: Result<Deserializable<Message<Body<Value>>>, _> = from_slice(&buf);
        let message = result.unwrap().0;
        assert!(message.header.is_some());
        assert!(message.delivery_annotations.is_none());
        assert!(message.message_annotations.is_none());
        assert!(message.properties.is_some());
        assert!(message.application_properties.is_none());
        assert!(message.body.is_empty());
        assert!(message.footer.is_none());
    }

    #[test]
    fn test_decoding_message_with_no_body_section_from_reader() {
        let buf: [u8; 8] = [0x0, 0x53, 0x70, 0x45, 0x0, 0x53, 0x73, 0x45];
        let result: Result<Deserializable<Message<Value>>, _> = from_reader(&buf[..]);
        let message = result.unwrap().0;
        assert!(message.header.is_some());
        assert!(message.delivery_annotations.is_none());
        assert!(message.message_annotations.is_none());
        assert!(message.properties.is_some());
        assert!(message.application_properties.is_none());
        assert!(matches!(message.body, Value::Null));
        assert!(message.footer.is_none());
    }

    #[test]
    fn test_encode_message_with_body_nothing() {
        let message: Message<AmqpValue<()>> = Message {
            header: None,
            delivery_annotations: None,
            message_annotations: None,
            properties: None,
            application_properties: None,
            body: AmqpValue(()),
            footer: None,
        };
        let serializable = Serializable(message);
        let buf = to_vec(&serializable).unwrap();

        let expected = Message::builder().value(()).build();
        let expected = to_vec(&Serializable(expected)).unwrap();

        assert_eq!(buf, expected);
    }

    #[test]
    fn test_decode_message_using_reader() {
        let buf = &[
            0x0, 0x53, 0x70, 0xc0, 0xb, 0x5, 0x40, 0x40, 0x70, 0x48, 0x19, 0x8, 0x0, 0x40, 0x52,
            0x3, 0x0, 0x53, 0x71, 0xc1, 0x24, 0x2, 0xa3, 0x10, 0x78, 0x2d, 0x6f, 0x70, 0x74, 0x2d,
            0x6c, 0x6f, 0x63, 0x6b, 0x2d, 0x74, 0x6f, 0x6b, 0x65, 0x6e, 0x98, 0xf4, 0xde, 0x71,
            0x99, 0x9f, 0x58, 0x41, 0x4e, 0xb6, 0x85, 0xd4, 0x27, 0x82, 0x92, 0x8f, 0xd0, 0x0,
            0x53, 0x72, 0xc1, 0x6c, 0x8, 0xa3, 0x13, 0x78, 0x2d, 0x6f, 0x70, 0x74, 0x2d, 0x65,
            0x6e, 0x71, 0x75, 0x65, 0x75, 0x65, 0x64, 0x2d, 0x74, 0x69, 0x6d, 0x65, 0x83, 0x0, 0x0,
            0x1, 0x82, 0x58, 0xd3, 0xcb, 0x78, 0xa3, 0x15, 0x78, 0x2d, 0x6f, 0x70, 0x74, 0x2d,
            0x73, 0x65, 0x71, 0x75, 0x65, 0x6e, 0x63, 0x65, 0x2d, 0x6e, 0x75, 0x6d, 0x62, 0x65,
            0x72, 0x55, 0x31, 0xa3, 0x12, 0x78, 0x2d, 0x6f, 0x70, 0x74, 0x2d, 0x6c, 0x6f, 0x63,
            0x6b, 0x65, 0x64, 0x2d, 0x75, 0x6e, 0x74, 0x69, 0x6c, 0x83, 0x0, 0x0, 0x1, 0x82, 0x58,
            0xdf, 0x5c, 0xf8, 0xa3, 0x13, 0x78, 0x2d, 0x6f, 0x70, 0x74, 0x2d, 0x6d, 0x65, 0x73,
            0x73, 0x61, 0x67, 0x65, 0x2d, 0x73, 0x74, 0x61, 0x74, 0x65, 0x54, 0x0, 0x0, 0x53, 0x73,
            0xc0, 0x2c, 0xd, 0xa1, 0xd, 0x41, 0x6d, 0x71, 0x70, 0x4e, 0x65, 0x74, 0x4c, 0x69, 0x74,
            0x65, 0x2d, 0x31, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x83, 0x0, 0x0, 0x1, 0x82,
            0xa0, 0xec, 0xd3, 0x78, 0x83, 0x0, 0x0, 0x1, 0x82, 0x58, 0xd3, 0xcb, 0x78, 0x40, 0x40,
            0x40, 0x0, 0x53, 0x75, 0xa0, 0xa, 0x6d, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65, 0x20, 0x23,
            0x31,
        ];
        let result: Result<Deserializable<Message<Data>>, _> = from_reader(&buf[..]);
        assert!(result.is_ok());
        let message = result.unwrap().0;
        assert!(message.body.0.len() > 0);
    }

    #[test]
    fn test_encode_message_builder_with_data_batch() {
        use serde_amqp::extensions::TransparentVec;

        let data = Data(Binary::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]));
        let data_batch = TransparentVec::new(vec![data.clone(), data.clone(), data.clone()]);
        let message = Message::builder()
            .header(Header::default())
            .data_batch(data_batch)
            .build();
        let buf = to_vec(&Serializable(message)).unwrap();
        let expected = &[
            0x0u8, 0x53, 0x70, 0x45, 0x0, 0x53, 0x75, 0xa0, 0x9, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7,
            0x8, 0x9, 0x0, 0x53, 0x75, 0xa0, 0x9, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0x0,
            0x53, 0x75, 0xa0, 0x9, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9,
        ];
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_encode_message_with_data_batch() {
        use serde_amqp::extensions::TransparentVec;

        let data = Data(Binary::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]));
        let data_batch = TransparentVec::new(vec![data.clone(), data.clone(), data.clone()]);
        let message = Message {
            header: Some(Header::default()),
            delivery_annotations: None,
            message_annotations: None,
            properties: None,
            application_properties: None,
            body: data_batch,
            footer: None,
        };
        let buf = to_vec(&Serializable(message)).unwrap();
        let expected = &[
            0x0u8, 0x53, 0x70, 0x45, 0x0, 0x53, 0x75, 0xa0, 0x9, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7,
            0x8, 0x9, 0x0, 0x53, 0x75, 0xa0, 0x9, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0x0,
            0x53, 0x75, 0xa0, 0x9, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9,
        ];
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_decode_message_with_data_batch() {
        let buf = &[
            0x0u8, 0x53, 0x70, 0x45, 0x0, 0x53, 0x75, 0xa0, 0x9, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7,
            0x8, 0x9, 0x0, 0x53, 0x75, 0xa0, 0x9, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0x0,
            0x53, 0x75, 0xa0, 0x9, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9,
        ];
        let message: Deserializable<Message<Batch<Data>>> = from_slice(&buf[..]).unwrap();

        let data = Data(Binary::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]));
        let data_batch = vec![data.clone(), data.clone(), data.clone()];
        let expected = Message::builder()
            .header(Header::default())
            .data_batch(data_batch)
            .build();
        assert_eq!(message.0, expected);
    }

    #[test]
    fn test_encode_message_with_sequence_batch() {
        use serde_amqp::extensions::TransparentVec;

        let batch = TransparentVec::new(vec![
            AmqpSequence::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
            AmqpSequence::new(vec![Value::Int(4), Value::Int(5), Value::Int(6)]),
            AmqpSequence::new(vec![Value::Int(7), Value::Int(8), Value::Int(9)]),
        ]);
        let message = Message::builder()
            .header(Header::default())
            .sequence_batch(batch)
            .build();
        let buf = to_vec(&Serializable(message)).unwrap();
        let expected = &[
            0x0u8, 0x53, 0x70, 0x45, 0x0, 0x53, 0x76, 0xc0, 0x07, 0x3, 0x54, 0x1, 0x54, 0x2, 0x54,
            0x3, 0x0, 0x53, 0x76, 0xc0, 0x07, 0x3, 0x54, 0x4, 0x54, 0x5, 0x54, 0x6, 0x0, 0x53,
            0x76, 0xc0, 0x07, 0x3, 0x54, 0x7, 0x54, 0x8, 0x54, 0x9,
        ];
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_decode_message_with_sequence_batch() {
        use serde_amqp::extensions::TransparentVec;

        let buf = &[
            0x0u8, 0x53, 0x70, 0x45, 0x0, 0x53, 0x76, 0xc0, 0x07, 0x3, 0x54, 0x1, 0x54, 0x2, 0x54,
            0x3, 0x0, 0x53, 0x76, 0xc0, 0x07, 0x3, 0x54, 0x4, 0x54, 0x5, 0x54, 0x6, 0x0, 0x53,
            0x76, 0xc0, 0x07, 0x3, 0x54, 0x7, 0x54, 0x8, 0x54, 0x9,
        ];
        let message: Deserializable<Message<Batch<AmqpSequence<i32>>>> =
            from_slice(&buf[..]).unwrap();

        let batch = TransparentVec::new(vec![
            AmqpSequence::new(vec![1i32, 2, 3]),
            AmqpSequence::new(vec![4i32, 5, 6]),
            AmqpSequence::new(vec![7i32, 8, 9]),
        ]);
        let expected = Message::builder()
            .header(Header::default())
            .sequence_batch(batch)
            .build();
        assert_eq!(message.0, expected);
    }
}
