use std::{fmt::Display, marker::PhantomData};

use serde::{
    de::{self, VariantAccess},
    Serialize,
};
use serde_amqp::Value;

use crate::messaging::{AmqpSequence, AmqpValue, Data};

use super::__private::{Deserializable, Serializable};

/// Only one section of Data and one section of AmqpSequence
/// is supported for now
#[derive(Debug, Clone, PartialEq)]
pub enum Body<T> {
    /// A data section contains opaque binary data
    Data(Data),
    /// A sequence section contains an arbitrary number of structured data elements
    Sequence(AmqpSequence<T>),
    /// An amqp-value section contains a single AMQP value
    Value(AmqpValue<T>),

    /// There is no body section at all
    ///
    /// The core specification states that **at least one** body section should be present in
    /// the message. However, this is not the way `proton` is implemented, and according to
    /// [PROTON-2574](https://issues.apache.org/jira/browse/PROTON-2574), the wording in the
    /// core specification was an unintended.
    Empty,
}

impl<T> Body<T> {
    /// Whether the body section is a [`Data`]
    pub fn is_data(&self) -> bool {
        matches!(self, Body::Data(_))
    }

    /// Whether the body section is a [`AmqpSequence`]
    pub fn is_sequence(&self) -> bool {
        matches!(self, Body::Sequence(_))
    }

    /// Whether the body section is a [`AmqpValue`]
    pub fn is_value(&self) -> bool {
        matches!(self, Body::Value(_))
    }

    /// Whether the body section is `Nothing`
    #[deprecated(since = "0.5.2", note = "Please use is_empty() instead")]
    pub fn is_nothing(&self) -> bool {
        matches!(self, Body::Empty)
    }

    /// Whether the body section is `Nothing`
    pub fn is_empty(&self) -> bool {
        matches!(self, Body::Empty)
    }
}

impl<T> Display for Body<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Body::Data(data) => write!(f, "{}", data),
            Body::Sequence(seq) => write!(f, "{}", seq),
            Body::Value(val) => write!(f, "{}", val),
            Body::Empty => write!(f, "Nothing"),
        }
    }
}

impl<T: Serialize> From<T> for Body<T> {
    fn from(value: T) -> Self {
        Self::Value(AmqpValue(value))
    }
}

impl<T: Serialize + Clone, const N: usize> From<[T; N]> for Body<T> {
    fn from(values: [T; N]) -> Self {
        Self::Sequence(AmqpSequence(values.to_vec()))
    }
}

impl<T> From<AmqpValue<T>> for Body<T> {
    fn from(value: AmqpValue<T>) -> Self {
        Self::Value(value)
    }
}

impl<T> From<AmqpSequence<T>> for Body<T> {
    fn from(val: AmqpSequence<T>) -> Self {
        Self::Sequence(val)
    }
}

impl From<Data> for Body<Value> {
    fn from(val: Data) -> Self {
        Self::Data(val)
    }
}

impl<T: Serialize> Serialize for Serializable<Body<T>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<T: Serialize> Serialize for Serializable<&Body<T>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T> de::Deserialize<'de> for Deserializable<Body<T>>
where
    T: de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Body::<T>::deserialize(deserializer)?;
        Ok(Deserializable(value))
    }
}

impl<T: Serialize> Body<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Body::Data(data) => Serializable(data).serialize(serializer),
            Body::Sequence(seq) => Serializable(seq).serialize(serializer),
            Body::Value(val) => Serializable(val).serialize(serializer),
            Body::Empty => Serializable(AmqpValue(())).serialize(serializer),
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
        formatter.write_str("Body variant. One of Vec<Data>, Vec<AmqpSequence>, AmqpValue")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v {
            "amqp:data:binary" => Ok(Field::Data),
            "amqp:amqp-sequence:list" => Ok(Field::Sequence),
            "amqp:amqp-value:*" => Ok(Field::Value),
            _ => Err(de::Error::custom("Invalid descriptor code")),
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
            _ => Err(de::Error::custom("Invalid descriptor code")),
        }
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

struct Visitor<T> {
    marker: PhantomData<T>,
}

impl<'de, T> de::Visitor<'de> for Visitor<T>
where
    T: de::Deserialize<'de>,
{
    type Value = Body<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum Body")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (val, variant) = data.variant()?;

        match val {
            Field::Data => {
                let Deserializable(data) = variant.newtype_variant()?;
                Ok(Body::Data(data))
            }
            Field::Sequence => {
                let Deserializable(sequence) = variant.newtype_variant()?;
                Ok(Body::Sequence(sequence))
            }
            Field::Value => {
                let Deserializable(value) = variant.newtype_variant()?;
                Ok(Body::Value(value))
            }
        }
    }
}

impl<'de, T> Body<T>
where
    T: de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_enum(
            serde_amqp::__constants::UNTAGGED_ENUM,
            &["Data", "Sequence", "Value"],
            Visitor {
                marker: PhantomData,
            },
        )
    }
}
