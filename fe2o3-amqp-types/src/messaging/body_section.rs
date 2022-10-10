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

#[cfg(docsrs)]
use super::{Data, AmqpSequence, Body, Batch};

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
pub trait IntoBody {
    /// Serializable
    type Body: SerializableBody;

    /// Convert to a `SerializbleBody`
    fn into_body(self) -> Self::Body;
}

/// Convert back to the type from a `DeserializableBody` which includes:
///
/// 1. [`AmqpValue`]
/// 2. [`AmqpSequence`]
/// 3. [`Data`]
/// 4. [`Body`]
/// 5. [`Batch<AmqpSequence>`]
/// 6. [`Batch<Data>`]
pub trait FromBody<'de>: Sized + FromEmptyBody {
    /// Deserializable body
    type Body: DeserializableBody<'de>;

    /// Convert back to the type from a `DeserializableBody`
    fn from_body(deserializable: Self::Body) -> Self;
}

/* -------------------------------------------------------------------------- */
/*                                  Option<T>                                 */
/* -------------------------------------------------------------------------- */

// Option can only be used on deserializing becuase the user should be certain
// on whether an empty body is going to be serialized
impl<'de, T> FromBody<'de> for Option<T>
where
    T: FromBody<'de>,
{
    type Body = T::Body;

    fn from_body(deserializable: Self::Body) -> Self {
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

impl IntoBody for Value {
    type Body = AmqpValue<Value>;

    fn into_body(self) -> Self::Body {
        AmqpValue(self)
    }
}

impl<'de> FromBody<'de> for Value {
    type Body = AmqpValue<Value>;

    fn from_body(deserializable: Self::Body) -> Self {
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

/* -------------------------- IntoBody -------------------------- */

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
        impl<$($generics: $bound),*> IntoBody for $type<$($generics),*> {
            type Body = AmqpValue<Self>;

            fn into_body(self) -> Self::Body {
                AmqpValue(self)
            }
        }
    };

    (AmqpValue, $type:ty) => {
        impl IntoBody for $type {
            type Body = AmqpValue<Self>;

            fn into_body(self) -> Self::Body {
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

impl<'a> IntoBody for &'a str {
    type Body = AmqpValue<&'a str>;

    fn into_body(self) -> Self::Body {
        AmqpValue(self)
    }
}

impl<'a> IntoBody for Cow<'a, str> {
    type Body = AmqpValue<Self>;

    fn into_body(self) -> Self::Body {
        AmqpValue(self)
    }
}

impl<'a> IntoBody for SymbolRef<'a> {
    type Body = AmqpValue<Self>;

    fn into_body(self) -> Self::Body {
        AmqpValue(self)
    }
}

impl<K, V> IntoBody for OrderedMap<K, V>
where
    K: ser::Serialize + std::hash::Hash + Eq,
    V: ser::Serialize,
{
    type Body = AmqpValue<Self>;

    fn into_body(self) -> Self::Body {
        AmqpValue(self)
    }
}

impl<K, V> IntoBody for HashMap<K, V>
where
    K: ser::Serialize + std::hash::Hash + Eq,
    V: ser::Serialize,
{
    type Body = AmqpValue<Self>;

    fn into_body(self) -> Self::Body {
        AmqpValue(self)
    }
}

impl<K, V> IntoBody for BTreeMap<K, V>
where
    K: ser::Serialize + Ord,
    V: ser::Serialize,
{
    type Body = AmqpValue<Self>;

    fn into_body(self) -> Self::Body {
        AmqpValue(self)
    }
}

/* ------------------------- FromBody ------------------------- */

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
        impl<'de, $($generics),*> FromBody<'de> for $type<$($generics),*>
        where
            $($generics: $bound<$lt>),*
        {
            type Body = AmqpValue<Self>;

            fn from_body(deserializable: Self::Body) -> Self {
                deserializable.0
            }
        }

        blanket_impl_from_empty_body!($($generics),*; $type);
    };

    (AmqpValue, $type:ty) => {
        impl<'de> FromBody<'de> for $type {
            type Body = AmqpValue<Self>;

            fn from_body(deserializable: Self::Body) -> Self {
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
impl<'de, K, V> FromBody<'de> for OrderedMap<K, V>
where
    K: de::Deserialize<'de> + std::hash::Hash + Eq,
    V: de::Deserialize<'de>,
{
    type Body = AmqpValue<Self>;

    fn from_body(deserializable: Self::Body) -> Self {
        deserializable.0
    }
}

blanket_impl_from_empty_body!(K, V; HashMap);
impl<'de, K, V> FromBody<'de> for HashMap<K, V>
where
    K: de::Deserialize<'de> + std::hash::Hash + Eq,
    V: de::Deserialize<'de>,
{
    type Body = AmqpValue<Self>;

    fn from_body(deserializable: Self::Body) -> Self {
        deserializable.0
    }
}

blanket_impl_from_empty_body!(K, V; BTreeMap);
impl<'de, K, V> FromBody<'de> for BTreeMap<K, V>
where
    K: de::Deserialize<'de> + Ord,
    V: de::Deserialize<'de>,
{
    type Body = AmqpValue<Self>;

    fn from_body(deserializable: Self::Body) -> Self {
        deserializable.0
    }
}
