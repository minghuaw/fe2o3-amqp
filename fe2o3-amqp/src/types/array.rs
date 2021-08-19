use serde::{Serialize, Deserialize};

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