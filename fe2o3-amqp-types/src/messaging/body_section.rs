use serde::{de, ser};
use serde_amqp::Value;

use self::sealed::Sealed;

use super::AmqpValue;

pub(crate) mod sealed {
    pub trait Sealed {}

    impl<T> Sealed for Option<T> where T: Sealed {}
}

/// Trait for message body
pub trait BodySection: Sealed + SerializableBody + DeserializableBody {}

impl<T> BodySection for T where T: Sealed + SerializableBody + DeserializableBody {}

/// Trait for a serializable body section
pub trait SerializableBody: Sealed {
    /// The serializable type
    type Serializable: ser::Serialize;

    /// Get a reference to the serializable type
    fn serializable(&self) -> &Self::Serializable;
}

impl<T> SerializableBody for T
where
    T: ser::Serialize + Sealed,
{
    type Serializable = Self;

    fn serializable(&self) -> &Self::Serializable {
        self
    }
}

/// Trait for a deserializable body section
pub trait DeserializableBody: Sealed {
    /// The deserializable type
    ///
    /// TODO: change to GAT once it stablizes
    type Deserializable: de::DeserializeOwned;

    /// Convert from deserializable to self
    fn from_deserializable(deserializable: Self::Deserializable) -> Self;
}

impl<T> DeserializableBody for T
where
    for<'de> T: de::Deserialize<'de> + Sealed,
{
    type Deserializable = Self;

    fn from_deserializable(deserializable: Self::Deserializable) -> Self {
        deserializable
    }
}

/// Convert the type to a `SerializableBody` which includes:
///
/// 1. [`AmqpValue`]
/// 2. [`AmqpSequence`]
/// 3. [`Data`]
/// 4. [`Body`]
/// 5. [`Batch<AmqpSequence>`]
/// 6. [`Batch<Data>`]
/// 7. [`Option<T>`] where `T` implements `SerializableBody`
pub trait IntoSerializableBody {
    /// Serializable
    type SerializableBody: SerializableBody;

    /// Convert to a `SerializbleBody`
    fn into_serializable_body(self) -> Self::SerializableBody;
}

impl<T> IntoSerializableBody for T where T: SerializableBody {
    type SerializableBody = Self;

    fn into_serializable_body(self) -> Self::SerializableBody {
        self
    }
}

/// Convert back to the type from a `DeserializableBody` which inclused:
///
/// 1. [`AmqpValue`]
/// 2. [`AmqpSequence`]
/// 3. [`Data`]
/// 4. [`Body`]
/// 5. [`Batch<AmqpSequence>`]
/// 6. [`Batch<Data>`]
/// 7. [`Option<T>`] where `T` implements `DeserializableBody`
pub trait FromDeserializableBody: Sized {
    /// Deserializable body
    type DeserializableBody: DeserializableBody;

    /// Convert back to the type from a `DeserializableBody`
    fn from_deserializable_body(deserializable: Self::DeserializableBody) -> Self;

    /// How to interprete an empty body
    fn from_empty_body<E: de::Error>() -> Result<Self, E> {
        Err(de::Error::custom("An empty body section is found. Consider use Message<Option<B>> or Message<Body<Value>> instead"))
    }
}

impl<T> FromDeserializableBody for T
where
    T: DeserializableBody,
{
    type DeserializableBody = Self;

    fn from_deserializable_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable
    } 
}
