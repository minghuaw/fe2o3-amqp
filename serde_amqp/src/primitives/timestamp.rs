use serde::de;
use serde::ser;

use crate::__constants::TIMESTAMP;

/// An absolute point in time
///
/// encoding name = "ms64", code = 0x83,
/// category = fixed, width = 8
/// label = "64-bit two’s-complement integer representing milliseconds since the unix epoch"
/// 64-bit two’s-complement integer representing milliseconds since the unix epoch
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Consume the wrapper into the inner i64
    pub fn into_inner(self) -> i64 {
        self.0
    }
}

impl From<i64> for Timestamp {
    fn from(val: i64) -> Self {
        Self(val)
    }
}

impl Timestamp {
    /// Creates a new [`Timestamp`] from milliseconds
    pub fn from_milliseconds(milliseconds: i64) -> Self {
        Self(milliseconds)
    }

    /// Get the timestamp value as milliseconds
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
}

impl<'de> de::Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(TIMESTAMP, Visitor {})
    }
}

/// Please note that this conversion does NOT check for overflow
#[cfg(feature = "time")]
impl From<time::OffsetDateTime> for Timestamp {
    fn from(val: time::OffsetDateTime) -> Self {
        Self((val.unix_timestamp_nanos() / 1_000_000) as i64)
    }
}

#[cfg(feature = "time")]
impl TryFrom<Timestamp> for time::OffsetDateTime {
    type Error = time::error::ComponentRange;

    fn try_from(value: Timestamp) -> Result<Self, Self::Error> {
        time::OffsetDateTime::from_unix_timestamp_nanos(value.0 as i128 * 1_000_000)
    }
}

/// Please note that this conversion does NOT check for overflow
#[cfg(feature = "time")]
impl From<time::Duration> for Timestamp {
    fn from(val: time::Duration) -> Self {
        Self(val.whole_milliseconds() as i64)
    }
}

#[cfg(feature = "time")]
impl From<Timestamp> for time::Duration {
    fn from(value: Timestamp) -> Self {
        time::Duration::milliseconds(value.0)
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::Duration> for Timestamp {
    fn from(val: chrono::Duration) -> Self {
        Self(val.num_milliseconds())
    }
}

#[cfg(feature = "chrono")]
impl From<Timestamp> for chrono::Duration {
    fn from(value: Timestamp) -> Self {
        chrono::Duration::milliseconds(value.0)
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::DateTime<chrono::Utc>> for Timestamp {
    fn from(val: chrono::DateTime<chrono::Utc>) -> Self {
        Self(val.timestamp_millis())
    }
}

#[cfg(feature = "chrono")]
impl From<Timestamp> for chrono::DateTime<chrono::Utc> {
    fn from(value: Timestamp) -> Self {
        chrono::DateTime::<chrono::Utc>::from_utc(
            chrono::NaiveDateTime::from_timestamp(
                value.0 / 1000,
                (value.0 % 1000) as u32 * 1_000_000,
            ),
            chrono::Utc,
        )
    }
}
