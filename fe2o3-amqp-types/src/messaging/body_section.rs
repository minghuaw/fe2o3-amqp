use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

use serde::{de, ser, Deserialize, Serialize};
use serde_amqp::{
    primitives::{
        Array, Binary, Dec128, Dec32, Dec64, OrderedMap, Symbol, SymbolRef, Timestamp, Uuid,
    },
    Value,
};

use self::__private::BodySection;

use super::AmqpValue;

pub(crate) mod __private {
    /// Marker trait for message body.
    ///
    /// This is only implemented for
    ///
    /// 1. [`Body`]
    /// 2. [`AmqpValue`]
    /// 3. [`AmqpSequnce`]
    /// 4. [`Data`]
    /// 5. [`Batch<AmqpSequence>`]
    /// 6. [`Batch<Data>`]
    pub trait BodySection {}
}

/// Marker trait for a serializable body section.
///
/// This is only implemented for
///
/// 1. [`Body`]
/// 2. [`AmqpValue`]
/// 3. [`AmqpSequnce`]
/// 4. [`Data`]
/// 5. [`Batch<AmqpSequence>`]
/// 6. [`Batch<Data>`]
pub trait SerializableBody: Serialize + BodySection {}

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
///
/// This is only implemented for
///
/// 1. [`Body`]
/// 2. [`AmqpValue`]
/// 3. [`AmqpSequnce`]
/// 4. [`Data`]
/// 5. [`Batch<AmqpSequence>`]
/// 6. [`Batch<Data>`]
pub trait DeserializableBody<'de>: Deserialize<'de> + BodySection {}

/// Convert the type to a `SerializableBody` which includes:
///
/// 1. [`AmqpValue`]
/// 2. [`AmqpSequence`]
/// 3. [`Data`]
/// 4. [`Body`]
/// 5. [`Batch<AmqpSequence>`]
/// 6. [`Batch<Data>`]
pub trait IntoSerializableBody {
    /// Serializable
    type SerializableBody: SerializableBody;

    /// Convert to a `SerializbleBody`
    fn into_body(self) -> Self::SerializableBody;
}

/// Convert back to the type from a `DeserializableBody` which includes:
///
/// 1. [`AmqpValue`]
/// 2. [`AmqpSequence`]
/// 3. [`Data`]
/// 4. [`Body`]
/// 5. [`Batch<AmqpSequence>`]
/// 6. [`Batch<Data>`]
pub trait FromDeserializableBody<'de>: Sized + FromEmptyBody {
    /// Deserializable body
    type DeserializableBody: DeserializableBody<'de>;

    /// Convert back to the type from a `DeserializableBody`
    fn from_body(deserializable: Self::DeserializableBody) -> Self;
}

/* -------------------------------------------------------------------------- */
/*                                  Option<T>                                 */
/* -------------------------------------------------------------------------- */

// Option can only be used on deserializing becuase the user should be certain
// on whether an empty body is going to be serialized
impl<'de, T> FromDeserializableBody<'de> for Option<T>
where
    T: FromDeserializableBody<'de>,
{
    type DeserializableBody = T::DeserializableBody;

    fn from_body(deserializable: Self::DeserializableBody) -> Self {
        Some(T::from_body(deserializable))
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

    fn into_body(self) -> Self::SerializableBody {
        AmqpValue(self)
    }
}

impl<'de> FromDeserializableBody<'de> for Value {
    type DeserializableBody = AmqpValue<Value>;

    fn from_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable.0
    }
}

impl FromEmptyBody for Value {
    type Error = serde_amqp::Error;

    fn from_empty_body() -> Result<Self, Self::Error> {
        Ok(Self::Null)
    }
}

/* -------------------------------------------------------------------------- */
/*                             Other common types                             */
/* -------------------------------------------------------------------------- */

/* -------------------------- IntoSerializableBody -------------------------- */

macro_rules! impl_into_serializable_body {
    (
        AmqpValue,
        {
            $($type:ty),*
        }
    ) => {
        $(impl_into_serializable_body!(AmqpValue, $type);)*
    };

    (AmqpValue, $($generics:ident: $bound:path),*; $type:tt) => {
        impl<$($generics: $bound),*> IntoSerializableBody for $type<$($generics),*> {
            type SerializableBody = AmqpValue<Self>;

            fn into_body(self) -> Self::SerializableBody {
                AmqpValue(self)
            }
        }
    };

    (AmqpValue, $type:ty) => {
        impl IntoSerializableBody for $type {
            type SerializableBody = AmqpValue<Self>;

            fn into_body(self) -> Self::SerializableBody {
                AmqpValue(self)
            }
        }
    };
}

impl_into_serializable_body! {
    AmqpValue,
    {
        (), bool, u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, char,
        Dec32, Dec64, Dec128, Timestamp, Uuid, Binary, String, Symbol
    }
}

impl_into_serializable_body!(AmqpValue, T:ser::Serialize; Vec);
impl_into_serializable_body!(AmqpValue, T:ser::Serialize; Array);

impl<'a> IntoSerializableBody for &'a str {
    type SerializableBody = AmqpValue<&'a str>;

    fn into_body(self) -> Self::SerializableBody {
        AmqpValue(self)
    }
}

impl<'a> IntoSerializableBody for Cow<'a, str> {
    type SerializableBody = AmqpValue<Self>;

    fn into_body(self) -> Self::SerializableBody {
        AmqpValue(self)
    }
}

impl<'a> IntoSerializableBody for SymbolRef<'a> {
    type SerializableBody = AmqpValue<Self>;

    fn into_body(self) -> Self::SerializableBody {
        AmqpValue(self)
    }
}

impl<K, V> IntoSerializableBody for OrderedMap<K, V>
where
    K: ser::Serialize + std::hash::Hash + Eq,
    V: ser::Serialize,
{
    type SerializableBody = AmqpValue<Self>;

    fn into_body(self) -> Self::SerializableBody {
        AmqpValue(self)
    }
}

impl<K, V> IntoSerializableBody for HashMap<K, V>
where
    K: ser::Serialize + std::hash::Hash + Eq,
    V: ser::Serialize,
{
    type SerializableBody = AmqpValue<Self>;

    fn into_body(self) -> Self::SerializableBody {
        AmqpValue(self)
    }
}

impl<K, V> IntoSerializableBody for BTreeMap<K, V>
where
    K: ser::Serialize + Ord,
    V: ser::Serialize,
{
    type SerializableBody = AmqpValue<Self>;

    fn into_body(self) -> Self::SerializableBody {
        AmqpValue(self)
    }
}

/* ------------------------- FromDeserializableBody ------------------------- */

macro_rules! blanket_impl_from_empty_body {
    ($($generics:ident),*; $type:tt) => {
        // Blanket impl simply returns an error
        impl<$($generics),*> FromEmptyBody for $type<$($generics),*>  {
            type Error = serde_amqp::Error;
        }
    };
    ($type:ty) => {
        // Blanket impl simply returns an error
        impl FromEmptyBody for $type {
            type Error = serde_amqp::Error;
        }
    };
}

macro_rules! impl_from_deserializable_or_empty_body {
    (
        AmqpValue,
        {
            $($type:ty),*
        }
    ) => {
        $(impl_from_deserializable_or_empty_body!(AmqpValue, $type);)*
    };

    (AmqpValue, $($generics:ident: $bound:ident<$lt:lifetime>),*; $type:tt) => {
        impl<'de, $($generics),*> FromDeserializableBody<'de> for $type<$($generics),*>
        where
            $($generics: $bound<$lt>),*
        {
            type DeserializableBody = AmqpValue<Self>;

            fn from_body(deserializable: Self::DeserializableBody) -> Self {
                deserializable.0
            }
        }

        blanket_impl_from_empty_body!($($generics),*; $type);
    };

    (AmqpValue, $type:ty) => {
        impl<'de> FromDeserializableBody<'de> for $type {
            type DeserializableBody = AmqpValue<Self>;

            fn from_body(deserializable: Self::DeserializableBody) -> Self {
                deserializable.0
            }
        }

        blanket_impl_from_empty_body!($type);
    };
}

impl_from_deserializable_or_empty_body! {
    AmqpValue,
    {
        (), bool, u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, char,
        Dec32, Dec64, Dec128, Timestamp, Uuid, Binary, String, Symbol
    }
}

impl_from_deserializable_or_empty_body!(AmqpValue, T:Deserialize<'de>; Vec);
impl_from_deserializable_or_empty_body!(AmqpValue, T:Deserialize<'de>; Array);

blanket_impl_from_empty_body!(K, V; OrderedMap);
impl<'de, K, V> FromDeserializableBody<'de> for OrderedMap<K, V>
where
    K: de::Deserialize<'de> + std::hash::Hash + Eq,
    V: de::Deserialize<'de>,
{
    type DeserializableBody = AmqpValue<Self>;

    fn from_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable.0
    }
}

blanket_impl_from_empty_body!(K, V; HashMap);
impl<'de, K, V> FromDeserializableBody<'de> for HashMap<K, V>
where
    K: de::Deserialize<'de> + std::hash::Hash + Eq,
    V: de::Deserialize<'de>,
{
    type DeserializableBody = AmqpValue<Self>;

    fn from_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable.0
    }
}

blanket_impl_from_empty_body!(K, V; BTreeMap);
impl<'de, K, V> FromDeserializableBody<'de> for BTreeMap<K, V>
where
    K: de::Deserialize<'de> + Ord,
    V: de::Deserialize<'de>,
{
    type DeserializableBody = AmqpValue<Self>;

    fn from_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable.0
    }
}
