//! Message ID

use serde::{
    de::{self, VariantAccess},
    Serialize,
};
use serde_amqp::{
    __constants::VALUE,
    format_code::EncodingCodes,
    primitives::{Binary, Ulong, Uuid},
};

/// Message ID
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum MessageId {
    /// 3.2.11 Message ID Ulong
    /// <type name="message-id-ulong" class="restricted" source="ulong" provides="message-id"/>
    Ulong(Ulong),

    /// 3.2.12 Message ID UUID
    /// <type name="message-id-uuid" class="restricted" source="uuid" provides="message-id"/>
    Uuid(Uuid),

    /// 3.2.13 Message ID Binary
    /// <type name="message-id-binary" class="restricted" source="binary" provides="message-id"/>
    Binary(Binary),

    /// 3.2.14 Message ID String
    /// <type name="message-id-string" class="restricted" source="string" provides="message-id"/>
    String(String),
}

impl From<u64> for MessageId {
    fn from(value: u64) -> Self {
        Self::Ulong(value)
    }
}

impl From<Uuid> for MessageId {
    fn from(value: Uuid) -> Self {
        Self::Uuid(value)
    }
}

impl From<Binary> for MessageId {
    fn from(value: Binary) -> Self {
        Self::Binary(value)
    }
}

impl From<String> for MessageId {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl Serialize for MessageId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            MessageId::Ulong(value) => value.serialize(serializer),
            MessageId::Uuid(value) => value.serialize(serializer),
            MessageId::Binary(value) => value.serialize(serializer),
            MessageId::String(value) => value.serialize(serializer),
        }
    }
}

enum Field {
    Ulong,
    Uuid,
    Binary,
    String,
}

struct FieldVisitor {}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("MessageId variant")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v
            .try_into()
            .map_err(|_| de::Error::custom("Unable to convert to EncodingCodes"))?
        {
            EncodingCodes::Ulong0 | EncodingCodes::SmallUlong | EncodingCodes::Ulong => {
                Ok(Field::Ulong)
            }
            EncodingCodes::Uuid => Ok(Field::Uuid),
            EncodingCodes::Vbin8 | EncodingCodes::Vbin32 => Ok(Field::Binary),
            EncodingCodes::Str8 | EncodingCodes::Str32 => Ok(Field::String),
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
    type Value = MessageId;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum MessageId")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (val, de) = data.variant()?;

        match val {
            Field::Ulong => {
                let val = de.newtype_variant()?;
                Ok(MessageId::Ulong(val))
            }
            Field::Uuid => {
                let val = de.newtype_variant()?;
                Ok(MessageId::Uuid(val))
            }
            Field::Binary => {
                let val = de.newtype_variant()?;
                Ok(MessageId::Binary(val))
            }
            Field::String => {
                let val = de.newtype_variant()?;
                Ok(MessageId::String(val))
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for MessageId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &[&str] = &["Ulong", "Uuid", "Binary", "String"];
        // VALUE will peek the format code
        deserializer.deserialize_enum(VALUE, VARIANTS, Visitor {})
    }
}

#[cfg(test)]
mod tests {
    use serde_amqp::{
        from_slice,
        primitives::{Binary, Uuid},
        to_vec,
    };

    use crate::messaging::MessageId;

    #[test]
    fn test_message_id_ulong() {
        let id = MessageId::Ulong(123456789);
        let buf = to_vec(&id).unwrap();
        let deserialized: MessageId = from_slice(&buf).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_message_id_uuid() {
        let id = MessageId::Uuid(Uuid::from([0u8; 16]));
        let buf = to_vec(&id).unwrap();
        let deserialized: MessageId = from_slice(&buf).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_message_id_binary() {
        let id = MessageId::Binary(Binary::from("amqp"));
        let buf = to_vec(&id).unwrap();
        let deserialized: MessageId = from_slice(&buf).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_message_id_string() {
        let id = MessageId::String(String::from("amqp"));
        let buf = to_vec(&id).unwrap();
        let deserialized: MessageId = from_slice(&buf).unwrap();
        assert_eq!(id, deserialized);
    }
}
