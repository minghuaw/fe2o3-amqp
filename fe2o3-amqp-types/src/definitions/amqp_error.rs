use std::{
    convert::{TryFrom, TryInto},
    fmt::{Debug, Display},
};

use serde::{de, ser};
use serde_amqp::primitives::Symbol;

use super::ErrorCondition;

/// Shared error conditions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmqpError {
    /// An internal error occurred. Operator intervention might be necessary to resume normal
    /// operation
    InternalError,

    /// A peer attempted to work with a remote entity that does not exist.
    NotFound,

    /// A peer attempted to work with a remote entity to which it has no access due to security
    /// settings
    UnauthorizedAccess,

    /// Data could not be decoded.
    DecodeError,

    /// A peer exceeded its resource allocation.
    ResourceLimitExceeded,

    /// The peer tried to use a frame in a manner that is inconsistent with the semantics defined in
    /// the specification.
    NotAllowed,

    /// The peer tried to use a frame in a manner that is inconsistent with the semantics defined in
    /// the specification.
    InvalidField,

    /// The peer tried to use functionality that is not implemented in its partner.
    NotImplemented,

    /// The client attempted to work with a server entity to which it has no access because another
    /// client is working with it
    ResourceLocked,

    /// The client made a request that was not allowed because some precondition failed.
    PreconditionFailed,

    /// A server entity the client is working with has been deleted.
    ResourceDeleted,

    /// The peer sent a frame that is not permitted in the current state.
    IllegalState,

    /// The peer cannot send a frame because the smallest encoding of the performative with the
    /// currently valid values would be too large to fit within a frame of the agreed maximum frame
    /// size. When transferring a message the message data can be sent in multiple transfer
    /// frames thereby avoiding this error. Similarly when attaching a link with a large unsettled
    /// map the endpoint MAY make use of the incomplete-unsettled flag to avoid the need for
    /// overly large frames
    FrameSizeTooSmall,
}

impl Display for AmqpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

impl std::error::Error for AmqpError {}

impl From<AmqpError> for ErrorCondition {
    fn from(err: AmqpError) -> Self {
        ErrorCondition::AmqpError(err)
    }
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

impl TryFrom<Symbol> for AmqpError {
    type Error = Symbol;

    fn try_from(value: Symbol) -> Result<Self, Self::Error> {
        match value.as_str().try_into() {
            Ok(val) => Ok(val),
            Err(_) => Err(value),
        }
    }
}

impl<'a> TryFrom<&'a str> for AmqpError {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let val = match value {
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
            _ => return Err(value),
        };

        Ok(val)
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

impl<'de> de::Deserialize<'de> for AmqpError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Symbol::deserialize(deserializer)?
            .try_into()
            .map_err(|_| de::Error::custom("Invalid symbol value for AmqpError"))
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use crate::definitions::AmqpError;
    use serde::de;
    use serde::ser;
    use serde_amqp::de::from_slice;
    use serde_amqp::format_code::EncodingCodes;
    use serde_amqp::ser::to_vec;

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
