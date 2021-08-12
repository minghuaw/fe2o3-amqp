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

/// 64-bit two’s-complement integer representing milliseconds since the unix epoch
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);
