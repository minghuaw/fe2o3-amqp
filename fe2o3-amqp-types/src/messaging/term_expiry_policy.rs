use std::convert::{TryFrom, TryInto};

use serde::{
    de::{self},
    ser,
};
use serde_amqp::primitives::Symbol;

/// 3.5.6 Terminus Expiry Policy
/// <type name="terminus-expiry-policy" class="restricted" source="symbol">
/// </type>
///
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub enum TerminusExpiryPolicy {
    /// <choice name="link-detach" value="link-detach"/>
    LinkDetach,

    /// <choice name="session-end" value="session-end"/>
    #[default]
    SessionEnd,

    /// <choice name="connection-close" value="connection-close"/>
    ConnectionClose,

    /// <choice name="never" value="never"/>
    Never,
}



impl From<&TerminusExpiryPolicy> for Symbol {
    fn from(value: &TerminusExpiryPolicy) -> Self {
        let val = match value {
            TerminusExpiryPolicy::LinkDetach => "link-detach",
            TerminusExpiryPolicy::SessionEnd => "session-end",
            TerminusExpiryPolicy::ConnectionClose => "connection-close",
            TerminusExpiryPolicy::Never => "never",
        };
        Symbol::from(val)
    }
}

impl<'a> TryFrom<&'a str> for TerminusExpiryPolicy {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let val = match value {
            "link-detach" => TerminusExpiryPolicy::LinkDetach,
            "session-end" => TerminusExpiryPolicy::SessionEnd,
            "connection-close" => TerminusExpiryPolicy::ConnectionClose,
            "never" => TerminusExpiryPolicy::Never,
            _ => return Err(value),
        };
        Ok(val)
    }
}

impl TryFrom<Symbol> for TerminusExpiryPolicy {
    type Error = Symbol;

    fn try_from(value: Symbol) -> Result<Self, Self::Error> {
        match value.as_str().try_into() {
            Ok(val) => Ok(val),
            Err(_) => Err(value),
        }
    }
}

impl ser::Serialize for TerminusExpiryPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Symbol::from(self).serialize(serializer)
    }
}

impl<'de> de::Deserialize<'de> for TerminusExpiryPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Symbol::deserialize(deserializer)?
            .try_into()
            .map_err(|_| de::Error::custom("Invalid symbol value for TerminusExpiryPolicy"))
    }
}
