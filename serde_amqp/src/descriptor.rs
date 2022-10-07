//! Definition of `Descriptor` type.

use crate::__constants::DESCRIPTOR;
use crate::primitives::Symbol;

/// Descriptor of a described type
///
/// How are descriptor name serilaized in other implementations?
/// 1. amqpnetlite: Symbol
/// 2. go-amqp: Symbol?
/// 3. qpid-proton-j2: Symbol
#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    // serde::Serialize, serde::Deserialize,
)]
// #[serde(untagged)]
pub enum Descriptor {
    /// A name descriptor
    Name(Symbol),
    /// A code descriptor
    Code(u64),
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
            EncodingCodes::ULong | EncodingCodes::ULong0 | EncodingCodes::SmallULong => {
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
        const VARIANTS: &[&str] = &["Name", "Code"];
        deserializer.deserialize_enum(DESCRIPTOR, VARIANTS, DescriptorVisitor {})
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PeekDescriptor {
    /// A name descriptor
    Name(Symbol),
    /// A code descriptor
    Code(u64),
}

struct PeekDescriptorVisitor {}

impl<'de> de::Visitor<'de> for PeekDescriptorVisitor {
    type Value = PeekDescriptor;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum Descriptor")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(PeekDescriptor::Name(Symbol::from(v)))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(PeekDescriptor::Name(Symbol::from(v)))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(PeekDescriptor::Code(v))
    }
}

impl<'de> de::Deserialize<'de> for PeekDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_identifier(PeekDescriptorVisitor {})
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::{
        de::Deserializer,
        descriptor::{Descriptor, PeekDescriptor},
        from_slice,
        read::SliceReader, primitives::Symbol,
    };

    #[test]
    fn test_deserialize_descriptor() {
        use crate::ser::to_vec;

        // let descriptor = Descriptor::Name(Symbol::from("amqp"));
        let descriptor = Descriptor::Code(113);
        let buf = to_vec(&descriptor).unwrap();
        println!("{:x?}", buf);
        let deserialized: Descriptor = from_slice(&buf).unwrap();
        assert_eq!(deserialized, descriptor)
    }

    #[test]
    fn test_peek_then_consume() {
        use crate::ser::to_vec;

        let descriptor = Descriptor::Code(113);
        let buf = to_vec(&descriptor).unwrap();

        let reader = SliceReader::new(&buf);
        let mut deserializer = Deserializer::new(reader);
        let peek = PeekDescriptor::deserialize(&mut deserializer).unwrap();
        assert_eq!(peek, PeekDescriptor::Code(113));

        let owned = Descriptor::deserialize(&mut deserializer).unwrap();
        assert_eq!(owned, descriptor);

        let should_fail = Descriptor::deserialize(&mut deserializer);
        assert!(should_fail.is_err());
    }

    #[test]
    fn test_peek_descriptor_code() {
        use crate::ser::to_vec;

        let descriptor = Descriptor::Code(113);
        let buf = to_vec(&descriptor).unwrap();

        let reader = SliceReader::new(&buf);
        let mut deserializer = Deserializer::new(reader);
        let peek = PeekDescriptor::deserialize(&mut deserializer).unwrap();
        assert_eq!(peek, PeekDescriptor::Code(113));
    }

    #[test]
    fn test_peek_descriptor_name() {
        use crate::ser::to_vec;

        let descriptor = Descriptor::Name(Symbol::from("test:name"));
        let buf = to_vec(&descriptor).unwrap();

        let reader = SliceReader::new(&buf);
        let mut deserializer = Deserializer::new(reader);
        let peek = PeekDescriptor::deserialize(&mut deserializer).unwrap();
        let expected = PeekDescriptor::Name(Symbol::from("test:name"));
        assert_eq!(peek, expected);
    }
}
