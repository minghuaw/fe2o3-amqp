use std::{
    convert::{TryFrom, TryInto},
    fmt::{Debug, Display},
};

use serde::{de, ser};
use serde_amqp::primitives::Symbol;

use super::ErrorCondition;

/// Symbols used to indicate connection error conditions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionError {
    /// An operator intervened to close the connection for some reason. The client could retry at
    /// some later date
    ConnectionForced,

    /// A valid frame header cannot be formed from the incoming byte stream.
    FramingError,

    /// The container is no longer available on the current connection. The peer SHOULD attempt
    /// reconnection to the container using the details provided in the info map.
    ///
    /// hostname: the hostname of the container. This is the value that SHOULD be
    /// supplied in the hostname field of the open frame, and during the
    /// SASL and TLS negotiation (if used).
    ///
    /// network-host: the DNS hostname or IP address of the machine hosting the container.
    ///
    /// port: the port number on the machine hosting the container
    Redirect,
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

impl std::error::Error for ConnectionError {}

impl From<ConnectionError> for ErrorCondition {
    fn from(err: ConnectionError) -> Self {
        ErrorCondition::ConnectionError(err)
    }
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

impl TryFrom<Symbol> for ConnectionError {
    type Error = Symbol;

    fn try_from(value: Symbol) -> Result<Self, Self::Error> {
        match value.as_str().try_into() {
            Ok(val) => Ok(val),
            Err(_) => Err(value),
        }
    }
}

impl<'a> TryFrom<&'a str> for ConnectionError {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let val = match value {
            "amqp:connection:forced" => ConnectionError::ConnectionForced,
            "amqp:connection:framing-error" => ConnectionError::FramingError,
            "amqp:connection:redirect" => ConnectionError::Redirect,
            _ => return Err(value),
        };
        Ok(val)
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

impl<'de> de::Deserialize<'de> for ConnectionError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Symbol::deserialize(deserializer)?
            .try_into()
            .map_err(|_| de::Error::custom("Invalid symbol value for SessionError"))
    }
}

#[cfg(test)]
mod tests {
    use serde_amqp::{de::from_slice, format_code::EncodingCodes, ser::to_vec};

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
