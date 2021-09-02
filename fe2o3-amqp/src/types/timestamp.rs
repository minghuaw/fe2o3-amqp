use serde::de;
use serde::ser;

pub const TIMESTAMP: &str = "TIMESTAMP";

/// 64-bit two’s-complement integer representing milliseconds since the unix epoch
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);

impl From<i64> for Timestamp {
    fn from(val: i64) -> Self {
        Self(val)
    }
}

impl Timestamp {
    pub fn from_milliseconds(milliseconds: i64) -> Self {
        Self(milliseconds)
    }

    pub fn milliseconds(&self) -> i64 {
        self.0
    }
}

impl ser::Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct(TIMESTAMP, &self.0)
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Timestamp;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct Timestamp")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Timestamp::from(v))
    }

    // fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    // where
    //     D: serde::Deserializer<'de>,
    // {
    //     let val: i64 = de::Deserialize::deserialize(deserializer)?;
    //     Ok(Timestamp::from(val))
    // }
}

impl<'de> de::Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(TIMESTAMP, Visitor {})
    }
}
