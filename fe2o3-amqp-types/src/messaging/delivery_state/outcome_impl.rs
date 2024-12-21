use serde::{
    de::{self, VariantAccess},
    ser,
};

use super::Outcome;

impl ser::Serialize for Outcome {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Outcome::Accepted(value) => value.serialize(serializer),
            Outcome::Rejected(value) => value.serialize(serializer),
            Outcome::Released(value) => value.serialize(serializer),
            Outcome::Modified(value) => value.serialize(serializer),
            #[cfg(feature = "transaction")]
            Outcome::Declared(value) => value.serialize(serializer),
        }
    }
}

enum Field {
    Accepted,
    Rejected,
    Released,
    Modified,
    #[cfg(feature = "transaction")]
    Declared,
}

struct FieldVisitor {}

impl de::Visitor<'_> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("variant identifier")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v {
            "amqp:accepted:list" => Field::Accepted,
            "amqp:rejected:list" => Field::Rejected,
            "amqp:released:list" => Field::Released,
            "amqp:modified:list" => Field::Modified,
            #[cfg(feature = "transaction")]
            "amqp:declared:list" => Field::Declared,
            _ => return Err(de::Error::custom("Wrong symbol value for descriptor")),
        };

        Ok(val)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v {
            0x0000_0000_0000_0024 => Field::Accepted,
            0x0000_0000_0000_0025 => Field::Rejected,
            0x0000_0000_0000_0026 => Field::Released,
            0x0000_0000_0000_0027 => Field::Modified,
            #[cfg(feature = "transaction")]
            0x0000_0000_0000_0033 => Field::Declared,
            _ => {
                return Err(de::Error::custom(format!(
                    "Wrong code value for descriptor, found {:#x?}",
                    v
                )))
            }
        };
        Ok(val)
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
    type Value = Outcome;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum DeliveryState")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (val, variant) = data.variant()?;

        match val {
            Field::Accepted => {
                let value = variant.newtype_variant()?;
                Ok(Outcome::Accepted(value))
            }
            Field::Rejected => {
                let value = variant.newtype_variant()?;
                Ok(Outcome::Rejected(value))
            }
            Field::Released => {
                let value = variant.newtype_variant()?;
                Ok(Outcome::Released(value))
            }
            Field::Modified => {
                let value = variant.newtype_variant()?;
                Ok(Outcome::Modified(value))
            }
            #[cfg(feature = "transaction")]
            Field::Declared => {
                let value = variant.newtype_variant()?;
                Ok(Outcome::Declared(value))
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for Outcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &[&str] = &[
            "amqp:accepted:list",
            "amqp:rejected:list",
            "amqp:released:list",
            "amqp:modified:list",
        ];
        deserializer.deserialize_enum("Outcome", VARIANTS, Visitor {})
    }
}
