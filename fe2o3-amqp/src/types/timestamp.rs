// use chrono::{DateTime, Utc};
// use serde::{Serialize, Deserialize};

// /// 64-bit two’s-complement integer representing milliseconds since the unix epoch
// ///
// /// TODO: Replace with i64?
// #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub struct Timestamp {
//     #[serde(with = "timestamp_format")]
//     pub timestamp: DateTime<Utc>
// }

// mod timestamp_format {
//     use chrono::{DateTime, Utc};
//     use serde::{self, Serializer, Deserializer};

//     // The signature of a serialize_with function must follow the pattern:
//     //
//     //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
//     //    where
//     //        S: Serializer
//     //
//     // although it may also be generic over the input types T.
//     pub fn serialize<S>(
//         datetime: &DateTime<Utc>,
//         serializer: S,
//     ) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         // let s = format!("{}", date.format(FORMAT));
//         // serializer.serialize_str(&s)
//         unimplemented!()
//     }

//     // The signature of a deserialize_with function must follow the pattern:
//     //
//     //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
//     //    where
//     //        D: Deserializer<'de>
//     //
//     // although it may also be generic over the output types T.
//     pub fn deserialize<'de, D>(
//         deserializer: D,
//     ) -> Result<DateTime<Utc>, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         // let s = String::deserialize(deserializer)?;
//         // Utc.datetime_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
//         unimplemented!()
//     }
// }

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

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val: i64 = de::Deserialize::deserialize(deserializer)?;
        Ok(Timestamp(val))
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
