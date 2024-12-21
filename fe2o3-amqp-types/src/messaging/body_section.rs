use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::Arc,
};

use serde::{de, ser, Deserialize, Serialize};
use serde_amqp::{
    lazy::LazyValue,
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
    use std::{rc::Rc, sync::Arc};

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
    /// 7. [`&T`] where `T` implements `BodySection`
    /// 8. [`&mut T`] where `T` implements `BodySection`
    /// 9. [`Arc<T>`] where `T` implements `BodySection`
    /// 10. [`Rc<T>`] where `T` implements `BodySection`
    /// 11. [`Box<T>`] where `T` implements `BodySection`
    pub trait BodySection {}

    impl<T> BodySection for &T where T: BodySection {}

    impl<T> BodySection for &mut T where T: BodySection {}

    impl<T> BodySection for Box<T> where T: BodySection {}

    impl<T> BodySection for Rc<T> where T: BodySection {}

    impl<T> BodySection for Arc<T> where T: BodySection {}
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
/// 7. [`&T`] where `T` implements `SerializableBody`
/// 8. [`&mut T`] where `T` implements `SerializableBody`
/// 9. [`Arc<T>`] where `T` implements `SerializableBody`
/// 10. [`Rc<T>`] where `T` implements `SerializableBody`
/// 11. [`Box<T>`] where `T` implements `SerializableBody`
pub trait SerializableBody: Serialize + BodySection {}

impl<T> SerializableBody for &T where T: SerializableBody {}

impl<T> SerializableBody for &mut T where T: SerializableBody {}

impl<T> SerializableBody for Box<T>
where
    T: SerializableBody,
    Box<T>: Serialize,
{
}

impl<T> SerializableBody for Rc<T>
where
    T: SerializableBody,
    Rc<T>: Serialize,
{
}

impl<T> SerializableBody for Arc<T>
where
    T: SerializableBody,
    Arc<T>: Serialize,
{
}

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
/// // Simply use the blanket implementation
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

impl<'a, T> IntoBody for &'a T
where
    T: IntoBody,
    T::Body: AsBodyRef<'a, T>,
{
    type Body = <T::Body as AsBodyRef<'a, T>>::BodyRef;

    fn into_body(self) -> Self::Body {
        <T::Body as AsBodyRef<'a, T>>::as_body_ref(self)
    }
}

/// Allows serializing with a reference to the type
pub trait AsBodyRef<'a, T>: SerializableBody + BodySection
where
    T: IntoBody<Body = Self>,
{
    /// Body reference
    type BodyRef: SerializableBody + 'a;

    /// Convert to a body reference
    fn as_body_ref(src: &'a T) -> Self::BodyRef;
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
/// impl FromBody<'_> for Foo {
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

impl FromBody<'_> for Value {
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

impl IntoBody for LazyValue {
    type Body = AmqpValue<Self>;

    fn into_body(self) -> Self::Body {
        AmqpValue(self)
    }
}

impl FromBody<'_> for LazyValue {
    type Body = AmqpValue<Self>;

    fn from_body(deserializable: Self::Body) -> Self {
        deserializable.0
    }
}

impl FromEmptyBody for LazyValue {
    fn from_empty_body() -> Result<Self, serde_amqp::Error> {
        serde_amqp::from_value(Value::Null)
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

impl IntoBody for Cow<'_, str> {
    type Body = AmqpValue<Self>;

    fn into_body(self) -> Self::Body {
        AmqpValue(self)
    }
}

impl IntoBody for SymbolRef<'_> {
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
        impl FromBody<'_> for $type {
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
