use fe2o3_amqp_types::{
    messaging::{AmqpValue, FromDeserializableBody, FromEmptyBody, IntoSerializableBody},
    transaction::{Declare, Discharge},
};
use serde::{
    de::{self, VariantAccess},
    ser,
};

#[derive(Debug)]
pub enum ControlMessageBody {
    Declare(Declare),
    Discharge(Discharge),
}

impl IntoSerializableBody for ControlMessageBody {
    type SerializableBody = AmqpValue<Self>;

    fn into_body(self) -> Self::SerializableBody {
        AmqpValue(self)
    }
}

impl FromEmptyBody for ControlMessageBody {
    type Error = serde_amqp::Error;
}

impl FromDeserializableBody for ControlMessageBody {
    type DeserializableBody = AmqpValue<Self>;

    fn from_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable.0
    }
}

impl ser::Serialize for ControlMessageBody {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ControlMessageBody::Declare(declare) => declare.serialize(serializer),
            ControlMessageBody::Discharge(discharge) => discharge.serialize(serializer),
        }
    }
}

enum Field {
    Declare,
    Discharge,
}

struct FieldVisitor {}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Control link message body variant identifier")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v {
            "amqp:declare:list" => Field::Declare,
            "amqp:discharge:list" => Field::Discharge,
            _ => return Err(de::Error::custom("Wrong symbol for descriptor")),
        };

        Ok(val)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v {
            0x0000_0000_0000_0031 => Field::Declare,
            0x0000_0000_0000_0032 => Field::Discharge,
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
    type Value = ControlMessageBody;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum ControlMessageBody")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (val, variant) = data.variant()?;

        match val {
            Field::Declare => {
                let value = variant.newtype_variant()?;
                Ok(ControlMessageBody::Declare(value))
            }
            Field::Discharge => {
                let value = variant.newtype_variant()?;
                Ok(ControlMessageBody::Discharge(value))
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for ControlMessageBody {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &[&str] = &["amqp:declare:list", "amqp:discharge:list"];
        deserializer.deserialize_enum("ControlMessageBody", VARIANTS, Visitor {})
    }
}
