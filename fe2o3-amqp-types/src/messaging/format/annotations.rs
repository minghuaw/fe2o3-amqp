//! Implements 3.2.10 Annotations

use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
};

use serde::{
    de::{self, VariantAccess},
    Serialize,
};
use serde_amqp::{
    format_code::EncodingCodes,
    primitives::{OrderedMap, Symbol, SymbolRef, ULong},
    Value,
    __constants::VALUE,
};

/// 3.2.10 Annotations
///
/// <type name="annotations" class="restricted" source="map"/>
///
/// The annotations type is a map where the keys are restricted to be of type symbol or of type
/// ulong. All ulong keys, and all symbolic keys except those beginning with “x-” are reserved. Keys
/// beginning with “x-opt-” MUST be ignored if not understood. On receiving an annotation key which
/// is not understood, and which does not begin with “x-opt”, the receiving AMQP container MUST
/// detach the link with a not-implemented error.
///
/// The key type allows looking up from the map using multiple types. The user may need to
/// explicitly coerce a type into a trait object
///
/// # Example
///
/// ```rust
/// use fe2o3_amqp_types::primitives::{Value, Symbol};
/// use fe2o3_amqp_types::messaging::annotations::{Annotations, OwnedKey, AnnotationKey};
///
/// let key = "key";
/// let val = Value::Symbol(Symbol::from("value"));
///
/// let mut annotations = Annotations::new();
/// annotations.insert(OwnedKey::from(key), val.clone());
///
/// let removed = annotations.remove(&key as &dyn AnnotationKey);
/// assert_eq!(removed, Some(val));
/// ```
pub type Annotations = OrderedMap<OwnedKey, Value>;

/// Key type for [`Annotations`]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OwnedKey {
    /// Symbol
    Symbol(Symbol),

    /// ULong
    ULong(ULong),
}

impl Default for OwnedKey {
    fn default() -> Self {
        Self::Symbol(Symbol::default())
    }
}

impl From<Symbol> for OwnedKey {
    fn from(value: Symbol) -> Self {
        Self::Symbol(value)
    }
}

impl<'a> From<SymbolRef<'a>> for OwnedKey {
    fn from(value: SymbolRef<'a>) -> Self {
        Self::Symbol(Symbol::from(value.0))
    }
}

impl From<u64> for OwnedKey {
    fn from(value: u64) -> Self {
        Self::ULong(value)
    }
}

impl From<String> for OwnedKey {
    fn from(value: String) -> Self {
        Self::Symbol(Symbol(value))
    }
}

impl<'a> From<&'a str> for OwnedKey {
    fn from(value: &'a str) -> Self {
        Self::Symbol(Symbol::from(value))
    }
}

impl Serialize for OwnedKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            OwnedKey::Symbol(value) => value.serialize(serializer),
            OwnedKey::ULong(value) => value.serialize(serializer),
        }
    }
}

enum Field {
    Symbol,
    ULong,
}

struct FieldVisitor {}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("OwnedKey variant")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v
            .try_into()
            .map_err(|_| de::Error::custom("Unable to convert to EncodingCodes"))?
        {
            EncodingCodes::Sym8 | EncodingCodes::Sym32 => Ok(Field::Symbol),
            EncodingCodes::ULong0 | EncodingCodes::SmallULong | EncodingCodes::ULong => {
                Ok(Field::ULong)
            }
            _ => Err(de::Error::custom("Invalid format code")),
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

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = OwnedKey;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("OwnedKey")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (val, de) = data.variant()?;

        match val {
            Field::Symbol => {
                let val = de.newtype_variant()?;
                Ok(OwnedKey::Symbol(val))
            }
            Field::ULong => {
                let val = de.newtype_variant()?;
                Ok(OwnedKey::ULong(val))
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for OwnedKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &[&str] = &["Symbol", "ULong"];
        deserializer.deserialize_enum(VALUE, VARIANTS, Visitor {})
    }
}

/// A borrowed version of [`OwnedKey`] which can be used to get entry/value from
/// [`Annotations`]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BorrowedKey<'k> {
    /// Symbol
    Symbol(SymbolRef<'k>),

    /// u64
    ULong(&'k u64),
}

/// This trait allows using different types as keys to get entry/value from [`Annotations`]
pub trait AnnotationKey {
    /// Returns a [`BorrowedKey`] that will be used to as the key
    fn key<'k>(&'k self) -> BorrowedKey<'k>;
}

impl<'a> AnnotationKey for BorrowedKey<'a> {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        self.clone()
    }
}

impl AnnotationKey for OwnedKey {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        match self {
            OwnedKey::Symbol(Symbol(s)) => BorrowedKey::Symbol(SymbolRef(s)),
            OwnedKey::ULong(v) => BorrowedKey::ULong(v),
        }
    }
}

impl AnnotationKey for u64 {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        BorrowedKey::ULong(self)
    }
}

impl AnnotationKey for str {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        BorrowedKey::Symbol(SymbolRef(self))
    }
}

impl AnnotationKey for &str {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        BorrowedKey::Symbol(SymbolRef(self))
    }
}

impl AnnotationKey for String {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        BorrowedKey::Symbol(SymbolRef(self))
    }
}

impl AnnotationKey for Symbol {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        BorrowedKey::Symbol(SymbolRef(&self.0))
    }
}

impl<'a> AnnotationKey for SymbolRef<'a> {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        BorrowedKey::Symbol(SymbolRef(self.0))
    }
}

impl<'a> Borrow<dyn AnnotationKey + 'a> for OwnedKey {
    fn borrow(&self) -> &(dyn AnnotationKey + 'a) {
        self
    }
}

impl<'a> PartialEq for (dyn AnnotationKey + 'a) {
    fn eq(&self, other: &Self) -> bool {
        self.key().eq(&other.key())
    }
}

impl<'a> Eq for (dyn AnnotationKey + 'a) {}

impl<'a> PartialOrd for (dyn AnnotationKey + 'a) {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.key().partial_cmp(&other.key())
    }
}

impl<'a> Ord for (dyn AnnotationKey + 'a) {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

impl<'a> Hash for (dyn AnnotationKey + 'a) {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state)
    }
}

#[cfg(test)]
mod tests {
    use serde_amqp::{
        from_slice,
        primitives::{Symbol, SymbolRef},
        to_vec, Value,
    };

    use crate::messaging::format::annotations::AnnotationKey;

    use super::Annotations;

    const STRING_KEY: &str = "string_key";
    const STR_KEY: &str = "str_key";
    const SYMBOL_KEY: &str = "symbol_key";
    const SYMBOL_REF_KEY: &str = "symbol_ref_key";
    const U64_KEY: &str = "u64_key";

    const STRING_VAL: &str = "string_val";
    const STR_VAL: &str = "str_val";
    const SYMBOL_VAL: &str = "symbol_val";
    const SYMBOL_REF_VAL: &str = "symbol_ref_val";
    const U64_VAL: u64 = 1000;

    fn string_val() -> Value {
        Value::String(String::from(STRING_VAL))
    }

    fn str_val() -> Value {
        Value::String(String::from(STR_VAL))
    }

    fn symbol_val() -> Value {
        Value::Symbol(Symbol::from(SYMBOL_VAL))
    }

    fn symbol_ref_val() -> Value {
        Value::Symbol(Symbol::from(SYMBOL_REF_VAL))
    }

    fn u64_val() -> Value {
        Value::ULong(U64_VAL)
    }

    fn create_annotations() -> Annotations {
        let mut annotations = Annotations::new();
        annotations.insert(STRING_KEY.into(), string_val());
        annotations.insert(STR_KEY.into(), str_val());
        annotations.insert(SYMBOL_KEY.into(), symbol_val());
        annotations.insert(SYMBOL_REF_KEY.into(), symbol_ref_val());
        annotations.insert(U64_KEY.into(), u64_val());
        annotations
    }

    fn create_annotations_with_different_order() -> Annotations {
        let mut annotations = Annotations::new();
        annotations.insert(U64_KEY.into(), u64_val());
        annotations.insert(SYMBOL_REF_KEY.into(), symbol_ref_val());
        annotations.insert(SYMBOL_KEY.into(), symbol_val());
        annotations.insert(STRING_KEY.into(), string_val());
        annotations.insert(STR_KEY.into(), str_val());
        annotations
    }

    #[test]
    fn test_annotations_with_different_order() {
        let annotations_1 = create_annotations();
        let annotations_2 = create_annotations_with_different_order();

        assert_ne!(annotations_1, annotations_2);
    }

    #[test]
    fn test_annotations_insert_with_different_key_types() {
        let annotations = create_annotations();

        assert_eq!(annotations.len(), 5);
    }

    #[test]
    fn test_annotations_get_with_different_key_types() {
        let mut annotations = create_annotations();

        let key = String::from(STRING_KEY);
        let val = annotations.remove(&key as &dyn AnnotationKey);
        assert_eq!(val, Some(string_val()));

        let key = STR_KEY;
        let val = annotations.remove(&key as &dyn AnnotationKey);
        assert_eq!(val, Some(str_val()));

        let key = Symbol::from(SYMBOL_KEY);
        let val = annotations.remove(&key as &dyn AnnotationKey);
        assert_eq!(val, Some(symbol_val()));

        let key = SymbolRef(SYMBOL_REF_KEY);
        let val = annotations.remove(&key as &dyn AnnotationKey);
        assert_eq!(val, Some(symbol_ref_val()));

        let key = U64_KEY;
        let val = annotations.remove(&key as &dyn AnnotationKey);
        assert_eq!(val, Some(u64_val()));

        assert!(annotations.is_empty())
    }

    #[test]
    fn test_serde_annotations() {
        let annotations = create_annotations();
        let buf = to_vec(&annotations).unwrap();
        let deserialized = from_slice(&buf).unwrap();
        assert_eq!(annotations, deserialized);
    }

    #[test]
    fn test_serde_annotations_with_different_order() {
        let annotations_1 = create_annotations();
        let annotations_2 = create_annotations_with_different_order();
        assert_ne!(annotations_1, annotations_2);

        let buf = to_vec(&annotations_1).unwrap();
        let deserialized: Annotations = from_slice(&buf).unwrap();
        assert_ne!(deserialized, annotations_2)
    }
}
