use serde::{de, ser};

use fe2o3_amqp::{constants::SYMBOL, primitives::Symbol};

#[derive(Debug, Clone, PartialEq)]
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
        Symbol::from(self).serialize(serializer)
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = LinkError;

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
            "amqp:link:detach-forced" => LinkError::DetachForced,
            "amqp:link:transfer-limit-exceeded" => LinkError::TransferLimitExceeded,
            "amqp:link:message-size-exceeded" => LinkError::MessageSizeExceeded,
            "amqp:link:redirect" => LinkError::Redirect,
            "amqp:link:stolen" => LinkError::Stolen,
            _ => return Err(de::Error::custom("Invalid symbol value for LinkError")),
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for LinkError {
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
    fn test_serialize_and_deserialzie_link_error() {
        let val = LinkError::DetachForced;
        let buf = to_vec(&val).unwrap();
        let val2: LinkError = from_slice(&buf).unwrap();
        assert_eq!(val, val2)
    }
}
