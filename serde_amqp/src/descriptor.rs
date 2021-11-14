use crate::__constants::DESCRIPTOR;
use crate::primitives::Symbol;

/// Descriptor of a described type
///
/// How are descriptor name serilaized in other implementations?
/// 1. amqpnetlite: Symbol
/// 2. go-amqp: Symbol?
/// 3. qpid-proton-j2: Symbol
#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, 
    // serde::Serialize, serde::Deserialize,
)]
// #[serde(untagged)]
pub enum Descriptor {
    Name(Symbol),
    Code(u64),
}

impl Descriptor {
    pub fn name(name: impl Into<Symbol>) -> Self {
        Self::Name(name.into())
    }

    pub fn code(code: u64) -> Self {
        Self::Code(code.into())
    }
}

use std::convert::TryInto;

use serde::de::{self, VariantAccess};
use serde::ser::Serialize;

use crate::format_code::EncodingCodes;

impl Serialize for Descriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Descriptor::Name(value) => {
                serializer.serialize_newtype_variant(DESCRIPTOR, 0, "Name", value)
            }
            Descriptor::Code(value) => {
                serializer.serialize_newtype_variant(DESCRIPTOR, 1, "Code", value)
            }
        }
    }
}

// Because the bytes will be consumed when `deserialize_identifier`
enum Field {
    // Name(Symbol),
    // Code(u64),
    Name,
    Code,
}

struct FieldVisitor {}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("variant identifier")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v
            .try_into()
            .map_err(|_| de::Error::custom("Unable to convert to EncodingCodes"))?
        {
            EncodingCodes::Sym32 | EncodingCodes::Sym8 => Ok(Field::Name),
            EncodingCodes::ULong | EncodingCodes::Ulong0 | EncodingCodes::SmallUlong => {
                Ok(Field::Code)
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

struct DescriptorVisitor {}

impl<'de> de::Visitor<'de> for DescriptorVisitor {
    type Value = Descriptor;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum Descriptor")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        println!(">>> Debug: visit_enum");
        let (val, de) = data.variant()?;
        match val {
            Field::Name => {
                let val = de.newtype_variant()?;
                Ok(Descriptor::Name(val))
            }
            Field::Code => {
                let val = de.newtype_variant()?;
                Ok(Descriptor::Code(val))
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for Descriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &'static [&'static str] = &["Name", "Code"];
        deserializer.deserialize_enum(DESCRIPTOR, VARIANTS, DescriptorVisitor {})
    }
}
