use std::{fmt::Display, marker::PhantomData};

use serde::{
    de::{self, VariantAccess},
    ser, Serialize,
};
use serde_amqp::{primitives::Binary, Value};

use crate::messaging::{
    AmqpSequence, AmqpValue, Batch, Data, DeserializableBody, FromBody, FromEmptyBody, IntoBody,
    SerializableBody, TransposeOption, __private::BodySection,
};

/// The body consists of one of the following three choices: one or more data sections, one or more
/// amqp-sequence sections, or a single amqp-value section.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Body<T> {
    /// An amqp-value section contains a single AMQP value
    Value(AmqpValue<T>),

    /// One or more data section
    ///
    /// Added since `"0.6.0"`
    Data(Batch<Data>),

    /// One or more sequence section
    ///
    /// Added since `"0.6.0"`
    Sequence(Batch<AmqpSequence<T>>),

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

    /// Whether the body section is [`Body::Empty`]
    pub fn is_empty(&self) -> bool {
        matches!(self, Body::Empty)
    }

    /// Consume the delivery into the body if the body is an [`AmqpValue`].
    /// An error will be returned if otherwise
    pub fn try_into_value(self) -> Result<T, Self> {
        match self {
            Body::Value(AmqpValue(value)) => Ok(value),
            _ => Err(self),
        }
    }

    /// Consume the delivery into the body if the body is one or more [`Data`].
    /// An error will be returned if otherwise
    pub fn try_into_data(self) -> Result<impl Iterator<Item = Binary>, Self> {
        match self {
            Body::Data(batch) => Ok(batch.into_iter().map(|data| data.0)),
            _ => Err(self),
        }
    }

    /// Consume the delivery into the body if the body is one or more [`AmqpSequence`].
    /// An error will be returned if otherwise
    pub fn try_into_sequence(self) -> Result<impl Iterator<Item = Vec<T>>, Self> {
        match self {
            Body::Sequence(batch) => Ok(batch.into_iter().map(|seq| seq.0)),
            _ => Err(self),
        }
    }

    /// Get a reference to the delivery body if the body is an [`AmqpValue`].
    /// An error will be returned if the body isnot an [`AmqpValue`]
    pub fn try_as_value(&self) -> Result<&T, &Self> {
        match self {
            Body::Value(AmqpValue(value)) => Ok(value),
            _ => Err(self),
        }
    }

    /// Get a reference to the delivery body if the body is one or more [`Data`].
    /// An error will be returned otherwise
    pub fn try_as_data(&self) -> Result<impl Iterator<Item = &Binary>, &Self> {
        match self {
            Body::Data(batch) => Ok(batch.iter().map(|data| &data.0)),
            _ => Err(self),
        }
    }

    /// Get a reference to the delivery body if the body is one or more [`AmqpSequence`].
    /// An error will be returned otherwise
    pub fn try_as_sequence(&self) -> Result<impl Iterator<Item = &Vec<T>>, &Self> {
        match self {
            Body::Sequence(batch) => Ok(batch.iter().map(|seq| &seq.0)),
            _ => Err(self),
        }
    }
}

impl<T> Display for Body<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Body::Value(val) => write!(f, "{}", val),
            Body::Data(_) => write!(f, "Data"),
            Body::Sequence(_) => write!(f, "Sequence"),
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
        Self::Sequence(Batch::new(vec![AmqpSequence(values.to_vec())]))
    }
}

impl<T> From<AmqpValue<T>> for Body<T> {
    fn from(value: AmqpValue<T>) -> Self {
        Self::Value(value)
    }
}

impl<T> From<AmqpSequence<T>> for Body<T> {
    fn from(val: AmqpSequence<T>) -> Self {
        Self::Sequence(Batch::new(vec![val]))
    }
}

impl From<Data> for Body<Value> {
    fn from(val: Data) -> Self {
        Self::Data(Batch::new(vec![val]))
    }
}

impl<T: Serialize> ser::Serialize for Body<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Body::Data(data) => data.serialize(serializer),
            Body::Sequence(seq) => seq.serialize(serializer),
            Body::Value(val) => val.serialize(serializer),
            Body::Empty => AmqpValue(()).serialize(serializer),
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
                let data: Batch<Data> = variant.newtype_variant()?;
                Ok(Body::Data(data))
            }
            Field::Sequence => {
                let sequence: Batch<AmqpSequence<_>> = variant.newtype_variant()?;
                Ok(Body::Sequence(sequence))
            }
            Field::Value => {
                let value = variant.newtype_variant()?;
                Ok(Body::Value(value))
            }
        }
    }
}

impl<'de, T> de::Deserialize<'de> for Body<T>
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

impl<T> BodySection for Body<T> {}

impl<T> SerializableBody for Body<T> where T: ser::Serialize {}

impl<'de, T> DeserializableBody<'de> for Body<T> where T: de::Deserialize<'de> {}

impl<T> IntoBody for Body<T>
where
    T: ser::Serialize,
{
    type Body = Self;

    fn into_body(self) -> Self::Body {
        self
    }
}

impl<'de, T> FromBody<'de> for Body<T>
where
    T: de::Deserialize<'de>,
{
    type Body = Self;

    fn from_body(deserializable: Self::Body) -> Self {
        deserializable
    }
}

impl<T> FromEmptyBody for Body<T> {
    fn from_empty_body() -> Result<Self, serde_amqp::Error> {
        Ok(Self::Empty)
    }
}

impl<'de, T, U> TransposeOption<'de, T> for Body<U>
where
    T: FromBody<'de, Body = Body<U>>,
    U: de::Deserialize<'de>,
{
    type From = Body<Option<U>>;

    fn transpose(src: Self::From) -> Option<T> {
        match src {
            Body::Value(AmqpValue(body)) => {
                body.map(|body| T::from_body(Body::Value(AmqpValue(body))))
            }
            Body::Data(batch) => {
                if batch.is_empty() {
                    // Note that a null value and a zero-length array (with a correct type for its
                    // elements) both describe an absence of a value and MUST be treated as
                    // semantically identical.
                    None
                } else {
                    Some(T::from_body(Body::Data(batch)))
                }
            }
            Body::Sequence(batch) => {
                if batch.is_empty() {
                    // Note that a null value and a zero-length array (with a correct type for its
                    // elements) both describe an absence of a value and MUST be treated as
                    // semantically identical.
                    None
                } else {
                    let batch: Option<Batch<AmqpSequence<U>>> = batch
                        .into_iter()
                        .map(|AmqpSequence(vec)| {
                            let vec: Option<Vec<U>> = vec.into_iter().collect();
                            vec.map(AmqpSequence)
                        })
                        .collect();

                    batch.map(|batch| T::from_body(Body::Sequence(batch)))
                }
            }
            Body::Empty => None,
        }
    }
}

#[cfg(test)]
mod tests {}
