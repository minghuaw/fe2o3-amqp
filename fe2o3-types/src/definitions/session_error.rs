use std::convert::{TryFrom, TryInto};

use serde::{de, ser};

use fe2o3_amqp::{constants::SYMBOL, primitives::Symbol};

#[derive(Debug, Clone, PartialEq)]
pub enum SessionError {
    WindowViolation,
    ErrantLink,
    HandleInUse,
    UnattachedHandle,
}

impl From<&SessionError> for Symbol {
    fn from(value: &SessionError) -> Self {
        let val = match value {
            &SessionError::WindowViolation => "amqp:session:window-violation",
            &SessionError::ErrantLink => "amqp:session:errant-link",
            &SessionError::HandleInUse => "amqp:session:handle-in-use",
            &SessionError::UnattachedHandle => "amqp:session:unattached-handle",
        };
        Symbol::from(val)
    }
}

impl TryFrom<Symbol> for SessionError {
    type Error = Symbol;

    fn try_from(value: Symbol) -> Result<Self, Self::Error> {
        match value.as_str().try_into() {
            Ok(val) => Ok(val),
            Err(_) => Err(value)
        }
    }
}

impl<'a> TryFrom<&'a str> for SessionError {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let val = match value {
            "amqp:session:window-violation" => SessionError::WindowViolation,
            "amqp:session:errant-link" => SessionError::ErrantLink,
            "amqp:session:handle-in-use" => SessionError::HandleInUse,
            "amqp:session:unattached-handle" => SessionError::UnattachedHandle,
            _ => return Err(value)
        };
        Ok(val)
    }
}

impl ser::Serialize for SessionError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Symbol::from(self).serialize(serializer)
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = SessionError;

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
        v.try_into()
            .map_err(|_| de::Error::custom("Invalid symbol value for SessionError"))
    }
}

impl<'de> de::Deserialize<'de> for SessionError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(SYMBOL, Visitor {})
    }
}

#[cfg(test)]
mod tests {

    use fe2o3_amqp::{de::from_slice, ser::to_vec};

    use super::*;

    #[test]
    fn test_serialize_and_deserialzie_session_error() {
        let val = SessionError::ErrantLink;
        let buf = to_vec(&val).unwrap();
        let val2: SessionError = from_slice(&buf).unwrap();
        assert_eq!(val, val2)
    }
}
