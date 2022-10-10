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
use super::{AmqpSequence, Batch, Body, Data};

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

/// This trait defines how to interprerte a message when there is an emtpy body.
///
/// It provides a blanket implementation that simply returns an error, which should work for most
/// scenarios. If a mixture of empty body and non-empty body are expected, though `Option<T>` could
/// be used (ie. `Delivery<Option<String>>`), it is recommended to use [`Body<Value>`] as the body
/// section type (ie. `Delivery<Body<Value>>`) as an empty body might be serialized differently in
/// different implementations.
///
/// # Example
///
/// ```rust
/// use fe2o3_amqp_types::messaging::FromEmptyBody;
/// 
/// struct Foo { a: i32 }
///
/// // Simply use the blanked implementation
/// impl FromEmptyBody for Foo {}
/// ```
///
/// # Mis-wording of in the core specification
///
/// Though the core specification states that **at least one** body section should be present in the
/// message. However, this is not the way `proton` is implemented, and according to
/// [PROTON-2574](https://issues.apache.org/jira/browse/PROTON-2574), the wording in the core
/// specification was an unintended.
pub trait FromEmptyBody: Sized {
    /// Interprete an empty body
    fn from_empty_body() -> Result<Self, serde_amqp::Error> {
        Err(de::Error::custom("An empty body section is found. Consider use Message<Option<B>> or Message<Body<Value>> instead"))
    }
}

/// A trait defining how to handle an optional body section.
pub trait TransposeOption<'de, To: FromBody<'de>>: BodySection + Sized {
    /// The optional body section type
    type From: DeserializableBody<'de>;

    /// Transpose from the optional body section to a `Oprion<To>`
    fn transpose(src: Self::From) -> Option<To>;
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
/// 7. `Option<B> where B: DeserializableBody`
pub trait DeserializableBody<'de>: Deserialize<'de> + BodySection {}

/// Convert the type to a `SerializableBody` which includes:
///
/// 1. [`AmqpValue`]
/// 2. [`AmqpSequence`]
/// 3. [`Batch<AmqpSequence>`]
/// 4. [`Data`]
/// 5. [`Batch<Data>`]
/// 6. [`Body`]
///
/// # Body section type (`IntoBody::Body`)?
///
/// 1. [`AmqpValue<T>`] would fit most use cases where the type `T` implements [`serde::Serialize`]
///    and is not an array.
/// 2. If there is an vector of items [`Vec<T>`] where type `T` implements [`serde::Serialize`], you
///    could choose either [`AmqpValue<Vec<T>>`] or [`AmqpSequence<T>`].
/// 3. If there is an array of array of element (ie. `Vec<Vec<T>>`) where `T` implements
///    [`serde::Serialize`], then [`Batch<AmqpSequence>`] is probably the best bet.
/// 4. If a `CustomType` needs to use a custom encoding format that is different from AMQP 1.0 (eg.
///    other serialization formats like json, bincode, etc.), then the type could be first
///    serialized into bytes (ie. `Vec<u8>`, &[u8], etc.) and choose the [`Data`] body section type.
/// 5. If there is an array of `CustomType`s that should be serialized with custom encoding formats,
///    then the [`Batch<Data>`] would probably be the best choice.
/// 6. Use [`Body`] if a type may need to change the body section type.
///
/// # Example
///
/// ```rust
/// use serde::Serialize;
/// use fe2o3_amqp_types::messaging::{IntoBody, AmqpValue};
/// 
/// #[derive(Serialize)]
/// struct Foo { a: i32 }
///
/// impl IntoBody for Foo {
///     type Body = AmqpValue<Foo>;
///
///     fn into_body(self) -> Self::Body {
///         AmqpValue(self)
///     }
/// }
/// ```
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
/// 3. [`Batch<AmqpSequence>`]
/// 4. [`Data`]
/// 5. [`Batch<Data>`]
/// 6. [`Body`]
///
/// Please note that this also requires the type to implement the [`FromEmptyBody`] trait, which
/// handles the case when an empty body is found. Please see [`FromEmptyBody`] for more information.
///
/// # `Body` section type?
/// 
/// # Example
/// 
/// ```rust
/// use serde::Deserialize;
/// use fe2o3_amqp_types::messaging::{AmqpValue, FromEmptyBody, FromBody};
/// 
/// #[derive(Deserialize)]
/// struct Foo {
///     a: i32
/// }
/// 
/// impl FromEmptyBody for Foo {}
/// 
/// impl<'de> FromBody<'de> for Foo {
///     type Body = AmqpValue<Self>;
/// 
///     fn from_body(body: Self::Body) -> Self {
///         body.0
///     }
/// }
/// ```
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
    T::Body: TransposeOption<'de, T>,
{
    type Body = <T::Body as TransposeOption<'de, T>>::From;

    fn from_body(deserializable: Self::Body) -> Self {
        <T::Body as TransposeOption<'de, T>>::transpose(deserializable)
    }
}

impl<B> BodySection for Option<B> where B: BodySection {}

impl<'de, B> DeserializableBody<'de> for Option<B> where B: DeserializableBody<'de> {}

impl<T> FromEmptyBody for Option<T> {
    fn from_empty_body() -> Result<Self, serde_amqp::Error> {
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
    fn from_empty_body() -> Result<Self, serde_amqp::Error> {
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
        impl<$($generics),*> FromEmptyBody for $type<$($generics),*>  { }
    };
    ($type:ty) => {
        // Blanket impl simply returns an error
        impl FromEmptyBody for $type { }
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

/* -------------------------------------------------------------------------- */
/*                                    Test                                    */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_amqp::{from_slice, to_vec, Value};

    use crate::messaging::{
        message::__private::{Deserializable, Serializable},
        Body, Message, AmqpValue,
    };

    use super::{IntoBody, FromEmptyBody};

    const TEST_STR: &str = "test_str";

    #[derive(Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
    struct TestExample {
        a: i32,
    }

    impl IntoBody for TestExample {
        type Body = AmqpValue<Self>;

        fn into_body(self) -> Self::Body {
            AmqpValue(self)
        }
    }

    impl FromEmptyBody for TestExample { }

    #[test]
    fn test_encoding_option_str() {
        // let should_not_work = Message::builder()
        //     .body(Some(TEST_STR)) // This should NOT work
        //     .build();
        // let buf = to_vec(&Serializable(should_not_work)).unwrap();

        let expected_msg = Message::builder().value(TEST_STR).build();
        let expected = to_vec(&Serializable(expected_msg)).unwrap();

        let msg = Message::builder().value(Some(TEST_STR)).build();
        let buf = to_vec(&Serializable(msg)).unwrap();

        assert_eq!(buf, expected);
    }

    #[test]
    fn test_decoding_some_str() {
        let src = Message::builder().value(TEST_STR).build();
        let buf = to_vec(&Serializable(src)).unwrap();
        let msg: Deserializable<Message<Option<String>>> = from_slice(&buf).unwrap();

        assert!(msg.0.body.is_some());
        assert_eq!(msg.0.body.unwrap(), TEST_STR);
    }

    #[test]
    fn test_decoding_none_str_with_body_section_value_null() {
        let src = Message::builder()
            // This actually serializes to `AmpqValue(Value::Null)`
            .body(Body::<Value>::Empty)
            .build();
        let buf = to_vec(&Serializable(src)).unwrap();
        // This will give an error if the empty body section is not encoded as a
        // `AmqpValue(Value::Null)`
        let msg: Deserializable<Message<Option<String>>> = from_slice(&buf).unwrap();

        assert!(msg.0.body.is_none());
    }

    #[test]
    fn test_decoding_none_str_with_no_body_section() {
        let buf: [u8; 8] = [0x0, 0x53, 0x70, 0x45, 0x0, 0x53, 0x73, 0x45];
        let msg: Deserializable<Message<Option<String>>> = from_slice(&buf).unwrap();
        assert!(msg.0.header.is_some());
        assert!(msg.0.body.is_none());
    }
}
