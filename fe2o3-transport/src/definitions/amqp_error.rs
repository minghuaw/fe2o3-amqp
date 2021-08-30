use fe2o3_amqp::types::Symbol;
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

impl AmqpError {
    fn value(&self) -> Symbol {
        let s = match self {
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
        let val = self.value();
        val.serialize(serializer)
    }
}

enum Field {
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
            "amqp:internal-error" => Field::InternalError,
            "amqp:not-found" => Field::NotFound,
            "amqp:unauthorized-access" => Field::UnauthorizedAccess,
            "amqp:decode-error" => Field::DecodeError,
            "amqp:resource-limit-exceeded" => Field::ResourceLimitExceeded,
            "amqp:not-allowed" => Field::NotAllowed,
            "amqp:invalid-field" => Field::InvalidField,
            "amqp:not-implemented" => Field::NotImplemented,
            "amqp:resource-locked" => Field::ResourceLocked,
            "amqp:precondition-failed" => Field::PreconditionFailed,
            "amqp:resource-deleted" => Field::ResourceDeleted,
            "amqp:illegal-state" => Field::IllegalState,
            "amqp:frame-size-too-small" => Field::FrameSizeTooSmall,
            _ => return Err(de::Error::custom("Invalid symbol value for AmqpError")),
        };

        Ok(val)   
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val: String = de::Deserialize::deserialize(deserializer)?;
        let val = match val.as_str() {
            "amqp:internal-error" => Field::InternalError,
            "amqp:not-found" => Field::NotFound,
            "amqp:unauthorized-access" => Field::UnauthorizedAccess,
            "amqp:decode-error" => Field::DecodeError,
            "amqp:resource-limit-exceeded" => Field::ResourceLimitExceeded,
            "amqp:not-allowed" => Field::NotAllowed,
            "amqp:invalid-field" => Field::InvalidField,
            "amqp:not-implemented" => Field::NotImplemented,
            "amqp:resource-locked" => Field::ResourceLocked,
            "amqp:precondition-failed" => Field::PreconditionFailed,
            "amqp:resource-deleted" => Field::ResourceDeleted,
            "amqp:illegal-state" => Field::IllegalState,
            "amqp:frame-size-too-small" => Field::FrameSizeTooSmall,
            _ => return Err(de::Error::custom("Invalid symbol value for AmqpError")),
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
    type Value = AmqpError;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum AmqpError")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (val, _) = data.variant()?;
        let val = match val {
            Field::InternalError => AmqpError::InternalError,
            Field::NotFound => AmqpError::NotFound,
            Field::UnauthorizedAccess => AmqpError::UnauthorizedAccess,
            Field::DecodeError => AmqpError::DecodeError,
            Field::ResourceLimitExceeded => AmqpError::ResourceLimitExceeded,
            Field::NotAllowed => AmqpError::NotAllowed,
            Field::InvalidField => AmqpError::InvalidField,
            Field::NotImplemented => AmqpError::NotImplemented,
            Field::ResourceLocked => AmqpError::ResourceDeleted,
            Field::PreconditionFailed => AmqpError::PreconditionFailed,
            Field::ResourceDeleted => AmqpError::ResourceDeleted,
            Field::IllegalState => AmqpError::IllegalState,
            Field::FrameSizeTooSmall => AmqpError::FrameSizeTooSmall,
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for AmqpError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &'static [&'static str] = &[
            "amqp:internal-error",
            "amqp:not-found",
            "amqp:unauthorized-access",
            "amqp:decode-error",
            "amqp:resource-limit-exceeded",
            "amqp:not-allowed",
            "amqp:invalid-field",
            "amqp:not-implemented",
            "amqp:resource-locked",
            "amqp:precondition-failed",
            "amqp:resource-deleted",
            "amqp:illegal-state",
            "amqp:frame-size-too-small",
        ];
        deserializer.deserialize_enum("AMQP_ERROR", VARIANTS, Visitor {})
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
