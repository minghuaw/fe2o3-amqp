use serde::{
    de::{self, Visitor},
    Serialize,
};

use crate::__constants::SYMBOL;

/// Symbol should only contain ASCII characters. The implementation, however, wraps
/// over a String. `AmqpNetLite` also wraps around a String, which in c# is utf-16.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(pub String);

impl Symbol {
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        &self.0[..]
    }
}

impl From<String> for Symbol {
    fn from(val: String) -> Self {
        Self(val)
    }
}

impl From<&str> for Symbol {
    fn from(val: &str) -> Self {
        Self(val.into())
    }
}

impl Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct(SYMBOL, &self.0)
    }
}

struct SymbolVisitor {}

impl<'de> Visitor<'de> for SymbolVisitor {
    type Value = Symbol;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct Symbol")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Symbol::from(v))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val: String = de::Deserialize::deserialize(deserializer)?;
        Ok(Symbol::new(val))
    }
}

impl<'de> de::Deserialize<'de> for Symbol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(SYMBOL, SymbolVisitor {})
    }
}
