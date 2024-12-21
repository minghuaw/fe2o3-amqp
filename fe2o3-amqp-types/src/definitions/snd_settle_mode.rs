use serde::{de, ser};

/// 2.8.2
/// sender receiver
/// Sender Settle Mode
/// Settlement policy for a sender.
/// <type name="sender-settle-mode" class="restricted" source="ubyte">
/// </type>
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SenderSettleMode {
    /// <choice name="unsettled" value="0"/>
    Unsettled,
    /// <choice name="settled" value="1"/>
    Settled,
    /// <choice name="mixed" value="2"/>
    #[default]
    Mixed,
}

impl From<SenderSettleMode> for u8 {
    fn from(mode: SenderSettleMode) -> Self {
        match mode {
            SenderSettleMode::Unsettled => 0,
            SenderSettleMode::Settled => 1,
            SenderSettleMode::Mixed => 2,
        }
    }
}

impl From<&SenderSettleMode> for u8 {
    fn from(mode: &SenderSettleMode) -> Self {
        match mode {
            SenderSettleMode::Unsettled => 0,
            SenderSettleMode::Settled => 1,
            SenderSettleMode::Mixed => 2,
        }
    }
}

impl ser::Serialize for SenderSettleMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        u8::from(self).serialize(serializer)
    }
}

struct Visitor {}

impl de::Visitor<'_> for Visitor {
    type Value = SenderSettleMode;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum SenderSettleMode")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let val = match v {
            0 => SenderSettleMode::Unsettled,
            1 => SenderSettleMode::Settled,
            2 => SenderSettleMode::Mixed,
            _ => return Err(de::Error::custom("Invalid value for SenderSettleMode")),
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for SenderSettleMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_u8(Visitor {})
    }
}
