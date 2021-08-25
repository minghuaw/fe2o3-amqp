use crate::types::Symbol;
use serde::de;
use serde::ser::Serialize;

pub const DESCRIPTOR: &str = "DESCRIPTOR";

/// Descriptor of a described type
///
/// How are descriptor name serilaized in other implementations?
/// 1. amqpnetlite: Symbol
/// 2. go-amqp: Symbol?
/// 3. qpid-proton-j2: Symbol
#[derive(Debug, PartialEq)]
// #[serde(rename(serialize = "DESCRIPTOR"))]
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
    Name(Symbol),
    Code(u64),
}

struct FieldVisitor {}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("variant identifier")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        // It has to be Descriptor::Name(Symbol) if visit_string is called
        let name = Symbol::from(v);
        Ok(Field::Name(name))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let name = Symbol::from(v);
        Ok(Field::Name(name))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Field::Code(v))
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
        let (val, _) = data.variant()?;
        match val {
            Field::Name(name) => Ok(Descriptor::Name(name)),
            Field::Code(code) => Ok(Descriptor::Code(code)),
        }
    }
}

impl<'de> de::Deserialize<'de> for Descriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &'static [&'static str] = &["A", "B"];
        deserializer.deserialize_enum(DESCRIPTOR, VARIANTS, DescriptorVisitor {})
    }
}
