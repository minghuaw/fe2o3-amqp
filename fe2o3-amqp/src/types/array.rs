
use std::marker::PhantomData;

use serde::{Serialize, Deserialize, de::{Visitor}};

pub const ARRAY: &str = "ARRAY";

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename(deserialize = "ARRAY"))]
pub struct Array<T>(pub Vec<T>);

impl<T> From<Vec<T>> for Array<T> {
    fn from(val: Vec<T>) -> Self {
        Self(val)
    }
}

impl<T: Serialize> Serialize for Array<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct(ARRAY, &self.0)
    }
}

// struct ArrayVisitor<T> { 
//     marker: PhantomData<T>,
// }

// impl<'de, T: Deserialize<'de>> Visitor<'de> for ArrayVisitor<T> {
//     type Value = Array<T>;

//     fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         formatter.write_str("struct Array")
//     }

//     fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
//     where
//         D: serde::Deserializer<'de>, 
//     {
//         let val: Vec<T> = Deserialize::deserialize(deserializer)?;
//         Ok(Array(val))
//     }

//     fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
//     where
//         A: serde::de::SeqAccess<'de>, 
//     {
//         match seq.next_element::<Vec<T>>()? {
//             Some(val) => val,
//             None => 
//         }
//     }
// }

