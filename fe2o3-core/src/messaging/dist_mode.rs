use fe2o3_amqp::types::{SYMBOL, Symbol};
use serde::{de, ser};

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
            DistributionMode::Copy => "copy"
        };

        Symbol::from(s)
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

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = DistributionMode;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("variant identifier")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
            E: de::Error, {
        self.visit_str(&v)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
            E: de::Error, {
        let val = match v {
            "move" => DistributionMode::Move,
            "copy" => DistributionMode::Copy,
            _ => return Err(de::Error::custom("Invalid symbol value for DistributionMode")),
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for DistributionMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
            D: serde::Deserializer<'de> {
        deserializer.deserialize_newtype_struct(SYMBOL,Visitor {})
    }
}

