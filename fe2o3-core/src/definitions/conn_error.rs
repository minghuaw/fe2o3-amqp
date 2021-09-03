use fe2o3_amqp::{constants::SYMBOL, primitives::Symbol};
use serde::{de, ser};

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionError {
    ConnectionForced,
    FramingError,
    Redirect,
}

impl From<&ConnectionError> for Symbol {
    fn from(value: &ConnectionError) -> Self {
        let val = match value {
            ConnectionError::ConnectionForced => "amqp:connection:forced",
            ConnectionError::FramingError => "amqp:connection:framing-error",
            ConnectionError::Redirect => "amqp:connection:redirect",
        };
        Symbol::from(val)
    }
}

impl ser::Serialize for ConnectionError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Symbol::from(self).serialize(serializer)
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = ConnectionError;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("variant identifier")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(v.as_str())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v {
            "amqp:connection:forced" => ConnectionError::ConnectionForced,
            "amqp:connection:framing-error" => ConnectionError::FramingError,
            "amqp:connection:redirect" => ConnectionError::Redirect,
            _ => {
                return Err(de::Error::custom(
                    "Invalud symbol value for ConnectionError",
                ))
            }
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for ConnectionError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(SYMBOL, Visitor {})
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
        let mut val = vec![EncodingCodes::Sym8 as u8, sym_buf.len() as u8];
        val.append(&mut sym_buf);
        let recovered: ConnectionError = from_slice(&val).unwrap();
        println!("{:?}", recovered);
    }
}
