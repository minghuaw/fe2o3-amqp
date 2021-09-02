use fe2o3_amqp::types::{SYMBOL, Symbol};
use serde::{de, ser};

#[derive(Debug, PartialEq)]
pub enum AmqpError {
    InternalError,
    NotFound,
    UnauthorizedAccess,
    DecodeError,
    ResourceLimitExceeded,
    NotAllowed,
    InvalidField,
    NotImplemented,
    ResourceLocked,
    PreconditionFailed,
    ResourceDeleted,
    IllegalState,
    FrameSizeTooSmall,
}

impl From<&AmqpError> for Symbol {
    fn from(value: &AmqpError) -> Self {
        let s = match value {
            AmqpError::InternalError => "amqp:internal-error",
            AmqpError::NotFound => "amqp:not-found",
            AmqpError::UnauthorizedAccess => "amqp:unauthorized-access",
            AmqpError::DecodeError => "amqp:decode-error",
            AmqpError::ResourceLimitExceeded => "amqp:resource-limit-exceeded",
            AmqpError::NotAllowed => "amqp:not-allowed",
            AmqpError::InvalidField => "amqp:invalid-field",
            AmqpError::NotImplemented => "amqp:not-implemented",
            AmqpError::ResourceLocked => "amqp:resource-locked",
            AmqpError::PreconditionFailed => "amqp:precondition-failed",
            AmqpError::ResourceDeleted => "amqp:resource-deleted",
            AmqpError::IllegalState => "amqp:illegal-state",
            AmqpError::FrameSizeTooSmall => "amqp:frame-size-too-small",
        };

        Symbol::from(s)
    }
}

impl ser::Serialize for AmqpError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let val = Symbol::from(self);
        val.serialize(serializer)
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = AmqpError;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum AmqpError")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&v)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v {
            "amqp:internal-error" => AmqpError::InternalError,
            "amqp:not-found" => AmqpError::NotFound,
            "amqp:unauthorized-access" => AmqpError::UnauthorizedAccess,
            "amqp:decode-error" => AmqpError::DecodeError,
            "amqp:resource-limit-exceeded" => AmqpError::ResourceLimitExceeded,
            "amqp:not-allowed" => AmqpError::NotAllowed,
            "amqp:invalid-field" => AmqpError::InvalidField,
            "amqp:not-implemented" => AmqpError::NotImplemented,
            "amqp:resource-locked" => AmqpError::ResourceLocked,
            "amqp:precondition-failed" => AmqpError::PreconditionFailed,
            "amqp:resource-deleted" => AmqpError::ResourceDeleted,
            "amqp:illegal-state" => AmqpError::IllegalState,
            "amqp:frame-size-too-small" => AmqpError::FrameSizeTooSmall,
            _ => return Err(de::Error::custom("Invalid symbol value for AmqpError")),
        };

        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for AmqpError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // deserializer.deserialize_identifier(Visitor {})
        deserializer.deserialize_newtype_struct(SYMBOL, Visitor{})
    }
}


#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use crate::definitions::AmqpError;
    use fe2o3_amqp::de::from_slice;
    use fe2o3_amqp::format_code::EncodingCodes;
    use fe2o3_amqp::ser::to_vec;
    use serde::de;
    use serde::ser;

    fn assert_eq_on_serialized_and_expected<T>(val: T, expected: Vec<u8>)
    where
        T: ser::Serialize + Debug + PartialEq,
    {
        let serialized = to_vec(&val).unwrap();
        assert_eq!(serialized, expected)
    }

    fn assert_eq_on_from_slice_and_expected<T>(val: Vec<u8>, expected: T)
    where
        T: de::DeserializeOwned + Debug + PartialEq,
    {
        let deserialized: T = from_slice(&val).unwrap();
        assert_eq!(deserialized, expected)
    }

    #[test]
    fn test_serialize_amqp_error() {
        let val = AmqpError::DecodeError;
        let mut sym_val = "amqp:decode-error".as_bytes().to_vec();
        let mut expected = vec![EncodingCodes::Sym8 as u8, sym_val.len() as u8];
        expected.append(&mut sym_val);
        assert_eq_on_serialized_and_expected(val, expected);
    }

    #[test]
    fn test_deserialize_amqp_error() {
        let mut sym_val = "amqp:unauthorized-access".as_bytes().to_vec();
        let mut val = vec![EncodingCodes::Sym8 as u8, sym_val.len() as u8];
        val.append(&mut sym_val);
        let expected = AmqpError::UnauthorizedAccess;
        assert_eq_on_from_slice_and_expected(val, expected);
    }
}
