use std::convert::TryFrom;

use serde::{de, ser};

use serde_amqp::primitives::Symbol;

#[cfg(feature = "transaction")]
use crate::transaction::TransactionError;

use super::{AmqpError, ConnectionError, LinkError, SessionError};

#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum ErrorCondition {
    AmqpError(AmqpError),
    ConnectionError(ConnectionError),
    SessionError(SessionError),
    LinkError(LinkError),
    Custom(Symbol),

    #[cfg(feature = "transaction")]
    TransactionError(TransactionError),
}

impl ser::Serialize for ErrorCondition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::AmqpError(err) => err.serialize(serializer),
            Self::ConnectionError(err) => err.serialize(serializer),
            Self::SessionError(err) => err.serialize(serializer),
            Self::LinkError(err) => err.serialize(serializer),
            Self::Custom(err) => err.serialize(serializer),
            
            #[cfg(feature = "transaction")]
            Self::TransactionError(err) => err.serialize(serializer),
        }
    }
}

// struct Visitor {}

// impl<'de> de::Visitor<'de> for Visitor {
//     type Value = ErrorCondition;

//     fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         formatter.write_str("enum ErrorCondition")
//     }

//     fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
//     where
//         E: de::Error,
//     {
//         self.visit_str(v.as_str())
//     }

//     fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
//     where
//         E: de::Error,
//     {
//         let v = match AmqpError::try_from(v) {
//             Ok(val) => return Ok(ErrorCondition::AmqpError(val)),
//             Err(e) => e,
//         };
//         let v = match ConnectionError::try_from(v) {
//             Ok(val) => return Ok(ErrorCondition::ConnectionError(val)),
//             Err(e) => e,
//         };
//         let v = match SessionError::try_from(v) {
//             Ok(val) => return Ok(ErrorCondition::SessionError(val)),
//             Err(e) => e,
//         };
//         let v = match LinkError::try_from(v) {
//             Ok(val) => return Ok(ErrorCondition::LinkError(val)),
//             Err(e) => e,
//         };
//         Ok(ErrorCondition::Custom(Symbol::from(v)))
//     }
// }

impl<'de> de::Deserialize<'de> for ErrorCondition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Symbol::deserialize(deserializer)?;
        let v = value.as_str();
        let v = match AmqpError::try_from(v) {
            Ok(val) => return Ok(ErrorCondition::AmqpError(val)),
            Err(e) => e,
        };
        let v = match ConnectionError::try_from(v) {
            Ok(val) => return Ok(ErrorCondition::ConnectionError(val)),
            Err(e) => e,
        };
        let v = match SessionError::try_from(v) {
            Ok(val) => return Ok(ErrorCondition::SessionError(val)),
            Err(e) => e,
        };
        let v = match LinkError::try_from(v) {
            Ok(val) => return Ok(ErrorCondition::LinkError(val)),
            Err(e) => e,
        };
        #[cfg(feature = "transaction")]
        let v = match TransactionError::try_from(v) {
            Ok(val) => return Ok(ErrorCondition::TransactionError(val)),
            Err(e) => e,
        };
        Ok(ErrorCondition::Custom(Symbol::from(v)))
    }
}

#[cfg(test)]
mod tests {
    use serde_amqp::{format_code::EncodingCodes, from_slice};

    use crate::definitions::AmqpError;

    use super::ErrorCondition;

    #[test]
    fn test_serde_error_condition() {
        let expected = ErrorCondition::AmqpError(AmqpError::DecodeError);
        let mut sym_val = "amqp:decode-error".as_bytes().to_vec();
        let mut buf = vec![EncodingCodes::Sym8 as u8, sym_val.len() as u8];
        buf.append(&mut sym_val);

        let deserialized: ErrorCondition = from_slice(&buf).unwrap();
        assert_eq!(expected, deserialized);
    }
}
