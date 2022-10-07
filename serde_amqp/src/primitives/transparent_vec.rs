//! Implement transparent vec

use std::marker::PhantomData;

use serde::{de, Serialize};

use crate::__constants::TRANSPARENT_VEC;

/// This is NOT a type defined in the AMQP 1.0 core protocol. Because `Vec<T>`
/// (or any type that needs `SerializeSeq` other than `Array`) is serialized as
/// a `List`, this simply provides an alternative that is serialized as a bare sequence
/// of elements.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransparentVec<T>(Vec<T>);

impl<T> TransparentVec<T> {
    /// Creates a new [`TransparentVec`]
    pub fn new(vec: impl Into<Vec<T>>) -> Self {
        Self(vec.into())
    }

    /// Consumes self and returns the wrapped `Vec`
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T> Serialize for TransparentVec<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct(TRANSPARENT_VEC, &self.0)
    }
}

struct Visitor<T> {
    marker: PhantomData<T>,
}

impl<'de, T: de::Deserialize<'de>> de::Visitor<'de> for Visitor<T> {
    type Value = TransparentVec<T>;

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
        }
        Ok(TransparentVec(vec))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec: Vec<T> = de::Deserialize::deserialize(deserializer)?;
        Ok(TransparentVec(vec))
    }
}

impl<'de, T> de::Deserialize<'de> for TransparentVec<T>
where
    T: de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(
            TRANSPARENT_VEC,
            Visitor {
                marker: PhantomData,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{format_code::EncodingCodes, from_slice, to_vec};

    use super::TransparentVec;

    #[test]
    fn test_serialize_transparent_vec() {
        let vec = TransparentVec::new(vec![true, false, true]);
        let buf = to_vec(&vec).unwrap();
        let expected = vec![
            EncodingCodes::BooleanTrue as u8,
            EncodingCodes::BooleanFalse as u8,
            EncodingCodes::BooleanTrue as u8,
        ];
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_deserialize_transparent_vec() {
        let buf = vec![
            EncodingCodes::BooleanTrue as u8,
            EncodingCodes::BooleanFalse as u8,
            EncodingCodes::BooleanTrue as u8,
        ];
        let vec: TransparentVec<bool> = from_slice(&buf).unwrap();
        let expected = TransparentVec::new(vec![true, false, true]);
        assert_eq!(vec, expected);
    }
}
