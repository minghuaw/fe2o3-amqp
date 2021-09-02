use serde::{de, ser};

use fe2o3_amqp::types::Symbol;

#[derive(Debug, PartialEq)]
pub enum LinkError {
    DetachForced,
    TransferLimitExceeded,
    MessageSizeExceeded,
    Redirect,
    Stolen,
}

impl From<&LinkError> for Symbol {
    fn from(value: &LinkError) -> Self {
        let val = match value {
            &LinkError::DetachForced => "amqp:link:detach-forced",
            &LinkError::TransferLimitExceeded => "amqp:link:transfer-limit-exceeded",
            &LinkError::MessageSizeExceeded => "amqp:link:message-size-exceeded",
            &LinkError::Redirect => "amqp:link:redirect",
            &LinkError::Stolen => "amqp:link:stolen",
        };
        Symbol::from(val)
    }
}

impl ser::Serialize for LinkError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Symbol::from(self)
            .serialize(serializer)
    }
}

enum Field {
    DetachForced,
    TransferLimitExceeded,
    MessageSizeExceeded,
    Redirect,
    Stolen,
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
            "amqp:link:detach-forced" => Field::DetachForced,
            "amqp:link:transfer-limit-exceeded" => Field::TransferLimitExceeded,
            "amqp:link:message-size-exceeded" => Field::MessageSizeExceeded,
            "amqp:link:redirect" => Field::Redirect,
            "amqp:link:stolen" => Field::Stolen,
            _ => return Err(de::Error::custom("Invalid symbol value for LinkError")),
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
    type Value = LinkError;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum LinkError")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (val, _) = data.variant()?;
        let val = match val {
            Field::DetachForced => LinkError::DetachForced,
            Field::TransferLimitExceeded => LinkError::TransferLimitExceeded,
            Field::MessageSizeExceeded => LinkError::MessageSizeExceeded,
            Field::Redirect => LinkError::Redirect,
            Field::Stolen => LinkError::Stolen,
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for LinkError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &'static [&'static str] = &[
            "amqp:link:detach-forced",
            "amqp:link:transfer-limit-exceeded",
            "amqp:link:message-size-exceeded",
            "amqp:link:redirect",
            "amqp:link:stolen",
        ];
        deserializer.deserialize_enum("LINK_ERROR", VARIANTS, Visitor {})
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp::{de::from_slice, ser::to_vec};

    use super::*;

    #[test]
    fn test_serialize_and_deserialzie_link_error() {
        let val = LinkError::DetachForced;
        let buf = to_vec(&val).unwrap();
        let val2: LinkError = from_slice(&buf).unwrap();
        assert_eq!(val, val2)
    }
}
