use fe2o3_amqp::types::Symbol;
use serde::{de, ser};

/// TODO: manually implement serialize and deserialize
#[derive(Debug, PartialEq)]
pub enum ConnectionError {
    ConnectionForced,
    FramingError,
    Redirect,
}

impl ConnectionError {
    fn value(&self) -> Symbol {
        let val = match self {
            Self::ConnectionForced => "amqp:connection:forced",
            Self::FramingError => "amqp:connection:framing-error",
            Self::Redirect => "amqp:connection:redirect"
        };
        Symbol::from(val)
    }
}

impl ser::Serialize for ConnectionError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        self.value().serialize(serializer)
    }
}

enum Field {
    ConnectionForced,
    FramingError,
    Redirect
}

struct FieldVisitor { }

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("variant identifier")
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
            D: serde::Deserializer<'de>, {
        let val: String = de::Deserialize::deserialize(deserializer)?;
        let val = match val.as_str() {
            "amqp:connection:forced" => Field::ConnectionForced,
            "amqp:connection:framing-error" => Field::FramingError,
            "amqp:connection:redirect" => Field::Redirect,
            _ => return Err(de::Error::custom("Invalud symbol value for ConnectionError"))
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
            D: serde::Deserializer<'de> {
        deserializer.deserialize_identifier(FieldVisitor { })
    }
}

struct Visitor { }

impl<'de> de::Visitor<'de> for Visitor {
    type Value = ConnectionError;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum ConnectionError")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
            A: de::EnumAccess<'de>, {
        let (val, _) = data.variant()?;
        let val = match val {
            Field::ConnectionForced => ConnectionError::ConnectionForced,
            Field::FramingError => ConnectionError::FramingError,
            Field::Redirect => ConnectionError::Redirect
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for ConnectionError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> 
    {
        const VARIANTS: &'static [&'static str] = &[
            "amqp:connection:forced",
            "amqp:connection:framing-error",
            "amqp:connection:redirect",
        ];
        deserializer.deserialize_enum("CONNECTION_ERROR", VARIANTS, Visitor { })
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp::{de::from_slice, format_code::EncodingCodes, ser::to_vec};

    use super::ConnectionError;

    #[test]
    fn test_serialize_connection_error() {
        let val = ConnectionError::ConnectionForced;
        let buf = to_vec(&val).unwrap();
        println!("{:x?}", buf);
    }

    #[test]
    fn test_deserialize_connection_error() {
        let mut sym_buf = "amqp:connection:redirect".as_bytes().to_vec();
        let mut val = vec![
            EncodingCodes::Sym8 as u8,
            sym_buf.len() as u8
        ];
        val.append(&mut sym_buf);
        let recovered: ConnectionError = from_slice(&val).unwrap();
        println!("{:?}", recovered);
    }
}