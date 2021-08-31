use std::marker::PhantomData;

use serde::{de, ser};

pub const ARRAY: &str = "ARRAY";

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
// #[serde(rename(deserialize = "ARRAY"))]
pub struct Array<T>(pub Vec<T>);

impl<T> From<Vec<T>> for Array<T> {
    fn from(val: Vec<T>) -> Self {
        Self(val)
    }
}

impl<T> From<Array<T>> for Vec<T> {
    fn from(val: Array<T>) -> Self {
        val.0
    }
}

impl<T> AsMut<Vec<T>> for Array<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T> Array<T> {
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T: ser::Serialize> ser::Serialize for Array<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct(ARRAY, &self.0)
    }
}

struct Visitor<T> { 
    marker: PhantomData<T>
}

impl<'de, T: de::Deserialize<'de>> de::Visitor<'de> for Visitor<T> {
    type Value = Array<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct Array")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>, 
    {
        let mut vec = Vec::new();
        while let Some(elem) = seq.next_element()? {
            vec.push(elem);
        };
        Ok(Array::from(vec))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>, 
    {
        let vec: Vec<T> = de::Deserialize::deserialize(deserializer)?;
        Ok(Array::from(vec))
    }
}

impl<'de, T: de::Deserialize<'de>> de::Deserialize<'de> for Array<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> 
    {
        deserializer.deserialize_newtype_struct(ARRAY, Visitor { marker: PhantomData })
    }
}