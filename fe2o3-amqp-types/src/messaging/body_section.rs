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

/// How to interprerte an emtpy body
pub trait FromEmptyBody: Sized {
    /// Error
    type Error: de::Error;

    /// Interprete an empty body
    fn from_empty_body() -> Result<Self, Self::Error> {
        Err(<Self::Error as de::Error>::custom("An empty body section is found. Consider use Message<Option<B>> or Message<Body<Value>> instead"))
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

/// Convert back to the type from a `DeserializableBody` which inclused:
///
/// 1. [`AmqpValue`]
/// 2. [`AmqpSequence`]
/// 3. [`Data`]
/// 4. [`Body`]
/// 5. [`Batch<AmqpSequence>`]
/// 6. [`Batch<Data>`]
/// 7. [`Option<T>`] where `T` implements `DeserializableBody`
pub trait FromDeserializableBody: Sized + FromEmptyBody {
    /// Deserializable body
    type DeserializableBody: DeserializableBody;

    /// Convert back to the type from a `DeserializableBody`
    fn from_deserializable_body(deserializable: Self::DeserializableBody) -> Self;
}

/* -------------------------------------------------------------------------- */
/*                                  Option<T>                                 */
/* -------------------------------------------------------------------------- */

// Option can only be used on deserializing becuase the user should be certain
// on whether an empty body is going to be serialized
impl<T> FromDeserializableBody for Option<T>
where
    T: FromDeserializableBody,
{
    type DeserializableBody = T::DeserializableBody;

    fn from_deserializable_body(deserializable: Self::DeserializableBody) -> Self {
        Some(T::from_deserializable_body(deserializable))
    }
}

impl<T> FromEmptyBody for Option<T> {
    type Error = serde_amqp::Error;

    fn from_empty_body() -> Result<Self, Self::Error> {
        Ok(Self::None)
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Value                                   */
/* -------------------------------------------------------------------------- */

impl IntoSerializableBody for Value {
    type SerializableBody = AmqpValue<Value>;

    fn into_serializable_body(self) -> Self::SerializableBody {
        AmqpValue(self)
    }
}

impl FromDeserializableBody for Value {
    type DeserializableBody = AmqpValue<Value>;

    fn from_deserializable_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable.0
    }
}

impl FromEmptyBody for Value {
    type Error = serde_amqp::Error;

    fn from_empty_body() -> Result<Self, Self::Error> {
        Ok(Self::Null)
    }
}
