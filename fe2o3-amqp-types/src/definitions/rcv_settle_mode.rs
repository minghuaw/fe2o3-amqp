use serde::{de, ser};

/// 2.8.3 Receiver Settle Mode
/// Settlement policy for a receiver.
/// <type name="receiver-settle-mode" class="restricted" source="ubyte">
/// </type>
#[derive(Debug, Clone, PartialEq)]
pub enum ReceiverSettleMode {
    /// <choice name="first" value="0"/>
    First,
    /// <choice name="second" value="1"/>
    Second,
}

impl Default for ReceiverSettleMode {
    fn default() -> Self {
        ReceiverSettleMode::First
    }
}

impl From<ReceiverSettleMode> for u8 {
    fn from(mode: ReceiverSettleMode) -> Self {
        match mode {
            ReceiverSettleMode::First => 0,
            ReceiverSettleMode::Second => 1,
        }
    }
}

impl From<&ReceiverSettleMode> for u8 {
    fn from(mode: &ReceiverSettleMode) -> Self {
        match mode {
            ReceiverSettleMode::First => 0,
            ReceiverSettleMode::Second => 1,
        }
    }
}

impl ser::Serialize for ReceiverSettleMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        u8::from(self).serialize(serializer)
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = ReceiverSettleMode;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum ReceiverSettleMode")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v {
            0 => ReceiverSettleMode::First,
            1 => ReceiverSettleMode::Second,
            _ => return Err(de::Error::custom("Invalid value for ReceiverSettleMode")),
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for ReceiverSettleMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_u8(Visitor {})
    }
}
