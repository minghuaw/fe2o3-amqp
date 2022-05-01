use serde::{
    de::{self, VariantAccess},
    ser,
};

use super::DeliveryState;

impl ser::Serialize for DeliveryState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            DeliveryState::Accepted(value) => value.serialize(serializer),
            DeliveryState::Rejected(value) => value.serialize(serializer),
            DeliveryState::Released(value) => value.serialize(serializer),
            DeliveryState::Modified(value) => value.serialize(serializer),
            DeliveryState::Received(value) => value.serialize(serializer),

            #[cfg(feature = "transaction")]
            DeliveryState::Declared(value) => value.serialize(serializer),

            #[cfg(feature = "transaction")]
            DeliveryState::TransactionalState(value) => value.serialize(serializer),
        }
    }
}

enum Field {
    Accepted,
    Rejected,
    Released,
    Modified,
    Received,

    #[cfg(feature = "transaction")]
    Declared,

    #[cfg(feature = "transaction")]
    TransactionalState,
}

struct FieldVisitor {}

impl<'de> de::Visitor<'de> for FieldVisitor {
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
            "amqp:received:list" => Field::Received,

            #[cfg(feature = "transaction")]
            "amqp:declared:list" => Field::Declared,

            #[cfg(feature = "transaction")]
            "amqp:transactional-state:list" => Field::TransactionalState,

            _ => return Err(de::Error::custom("Wrong symbol value for descriptor")),
        };

        Ok(val)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v {
            0x0000_0000_0000_0023 => Field::Received,
            0x0000_0000_0000_0024 => Field::Accepted,
            0x0000_0000_0000_0025 => Field::Rejected,
            0x0000_0000_0000_0026 => Field::Released,
            0x0000_0000_0000_0027 => Field::Modified,

            #[cfg(feature = "transaction")]
            0x0000_0000_0000_0033 => Field::Declared,

            #[cfg(feature = "transaction")]
            0x0000_0000_0000_0034 => Field::TransactionalState,

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
    type Value = DeliveryState;

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
                Ok(DeliveryState::Accepted(value))
            }
            Field::Rejected => {
                let value = variant.newtype_variant()?;
                Ok(DeliveryState::Rejected(value))
            }
            Field::Released => {
                let value = variant.newtype_variant()?;
                Ok(DeliveryState::Released(value))
            }
            Field::Modified => {
                let value = variant.newtype_variant()?;
                Ok(DeliveryState::Modified(value))
            }
            Field::Received => {
                let value = variant.newtype_variant()?;
                Ok(DeliveryState::Received(value))
            }

            #[cfg(feature = "transaction")]
            Field::Declared => {
                let value = variant.newtype_variant()?;
                Ok(DeliveryState::Declared(value))
            }

            #[cfg(feature = "transaction")]
            Field::TransactionalState => {
                let value = variant.newtype_variant()?;
                Ok(DeliveryState::TransactionalState(value))
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for DeliveryState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &'static [&'static str] = &[
            "amqp:accepted:list",
            "amqp:rejected:list",
            "amqp:released:list",
            "amqp:modified:list",
            "amqp:received:list",
        ];
        deserializer.deserialize_enum("DeliveryState", VARIANTS, Visitor {})
    }
}
