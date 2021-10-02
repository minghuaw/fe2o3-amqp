use std::convert::{TryFrom, TryInto};

use fe2o3_amqp::{primitives::Symbol};
use serde::{de::{self}, ser};

/// 3.5.7 Standard Distribution Mode
/// Link distribution policy.
/// <type name="std-dist-mode" class="restricted" source="symbol" provides="distribution-mode">
/// </type>
///
#[derive(Debug, Clone)]
pub enum DistributionMode {
    /// <choice name="move" value="move"/>
    Move,
    /// <choice name="copy" value="copy"/>
    Copy,
}

impl From<&DistributionMode> for Symbol {
    fn from(v: &DistributionMode) -> Self {
        let s = match v {
            DistributionMode::Move => "move",
            DistributionMode::Copy => "copy",
        };

        Symbol::from(s)
    }
}

impl<'a> TryFrom<&'a str> for DistributionMode {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let val = match value {
            "move" => DistributionMode::Move,
            "copy" => DistributionMode::Copy,
            _ => return Err(value)
        };
        Ok(val)
    }
}

impl TryFrom<Symbol> for DistributionMode {
    type Error = Symbol;

    fn try_from(value: Symbol) -> Result<Self, Self::Error> {
        match value.as_str().try_into() {
            Ok(val) => Ok(val),
            Err(_) => Err(value),
        }
    }
}

impl ser::Serialize for DistributionMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let val = Symbol::from(self);
        val.serialize(serializer)
    }
}

impl<'de> de::Deserialize<'de> for DistributionMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Symbol::deserialize(deserializer)?
            .try_into()
            .map_err(|_| de::Error::custom("Invalid symbol value for DistributionMode"))
    }
}
