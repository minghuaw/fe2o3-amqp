use std::convert::{TryFrom, TryInto};

use serde::{de, ser};

use serde_amqp::primitives::Symbol;

use super::ErrorCondition;

#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum LinkError {
    DetachForced,
    TransferLimitExceeded,
    MessageSizeExceeded,
    Redirect,
    Stolen,
}

impl From<LinkError> for ErrorCondition {
    fn from(err: LinkError) -> Self {
        ErrorCondition::LinkError(err)
    }
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

impl TryFrom<Symbol> for LinkError {
    type Error = Symbol;

    fn try_from(value: Symbol) -> Result<Self, Self::Error> {
        match value.as_str().try_into() {
            Ok(val) => Ok(val),
            Err(_) => Err(value),
        }
    }
}

impl<'a> TryFrom<&'a str> for LinkError {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let val = match value {
            "amqp:link:detach-forced" => LinkError::DetachForced,
            "amqp:link:transfer-limit-exceeded" => LinkError::TransferLimitExceeded,
            "amqp:link:message-size-exceeded" => LinkError::MessageSizeExceeded,
            "amqp:link:redirect" => LinkError::Redirect,
            "amqp:link:stolen" => LinkError::Stolen,
            _ => return Err(value),
        };
        Ok(val)
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

impl<'de> de::Deserialize<'de> for LinkError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Symbol::deserialize(deserializer)?
            .try_into()
            .map_err(|_| de::Error::custom("Invalid symbol value for LinkError"))
    }
}

#[cfg(test)]
mod tests {
    use serde_amqp::{de::from_slice, ser::to_vec};

    use super::*;

    #[test]
    fn test_serialize_and_deserialzie_link_error() {
        let val = LinkError::DetachForced;
        let buf = to_vec(&val).unwrap();
        let val2: LinkError = from_slice(&buf).unwrap();
        assert_eq!(val, val2)
    }
}
