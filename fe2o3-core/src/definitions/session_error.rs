use serde::{de, ser};

use fe2o3_amqp::types::Symbol;

#[derive(Debug, PartialEq)]
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

impl ser::Serialize for SessionError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Symbol::from(self)
            .serialize(serializer)
    }
}

enum Field {
    WindowViolation,
    ErrantLink,
    HandleInUse,
    UnattachedHandle,
}

struct FieldVisitor {}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

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
            "amqp:session:window-violation" => Field::WindowViolation,
            "amqp:session:errant-link" => Field::ErrantLink,
            "amqp:session:handle-in-use" => Field::HandleInUse,
            "amqp:session:unattached-handle" => Field::UnattachedHandle,
            _ => return Err(de::Error::custom("Invalid symbol value for SessionError")),
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
    type Value = SessionError;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum SessionError")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (val, _) = data.variant()?;
        let val = match val {
            Field::WindowViolation => SessionError::WindowViolation,
            Field::ErrantLink => SessionError::ErrantLink,
            Field::HandleInUse => SessionError::HandleInUse,
            Field::UnattachedHandle => SessionError::UnattachedHandle,
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for SessionError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &'static [&'static str] = &[
            "amqp:session:window-violation",
            "amqp:session:errant-link",
            "amqp:session:handle-in-use",
            "amqp:session:unattached-handle",
        ];
        deserializer.deserialize_enum("SESSION_ERROR", VARIANTS, Visitor {})
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
