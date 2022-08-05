use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use serde::{
    de::{self, VariantAccess},
    ser,
};

use crate::{__constants::ARRAY, format_code::EncodingCodes};

/// A sequence of values of a single type.
///
/// encoding name = "array8", encoding code = 0xe0
/// category = array, width = 1,
/// label="up to 2^8 - 1 array elements with total size less than 2^8 octets"
///
/// encoding name = "array32", encoding code = 0xf0,
/// category = array, width = 4
/// label="up to 2^32 - 1 array elements with total size less than 2^32 octets"
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

impl<T> Deref for Array<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Array<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Array<T> {
    /// Consumes the wrapper into the inner vector
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

enum Field {
    Single,
    Multiple,
}

struct FieldVisitor {}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Single or Multiple identifier for Array")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v
            .try_into()
            .map_err(|_| de::Error::custom("Unable to convert to EncodingCodes"))?
        {
            EncodingCodes::Array8 | EncodingCodes::Array32 => Ok(Field::Multiple),
            _ => Ok(Field::Single),
        }
    }
}

impl<'de> de::Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_identifier(FieldVisitor {})
    }
}

struct Visitor<T> {
    marker: PhantomData<T>,
}

impl<'de, T: de::Deserialize<'de>> de::Visitor<'de> for Visitor<T> {
    type Value = Array<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct Array")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (val, de) = data.variant()?;
        match val {
            Field::Single => {
                let val: T = de.newtype_variant()?;
                Ok(Array(vec![val]))
            }
            Field::Multiple => {
                let vec: Vec<T> = de.newtype_variant()?;
                Ok(Array(vec))
            }
        }
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut vec = Vec::new();
        while let Some(elem) = seq.next_element()? {
            vec.push(elem);
        }
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
        D: serde::Deserializer<'de>,
    {
        // deserializer.deserialize_newtype_struct(
        //     ARRAY,
        //     Visitor {
        //         marker: PhantomData,
        //     },
        // )
        const VARIANTS: &[&str] = &["Single", "Multiple"];
        deserializer.deserialize_enum(
            ARRAY,
            VARIANTS,
            Visitor {
                marker: PhantomData,
            },
        )
    }
}

impl<T> FromIterator<T> for Array<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let v = Vec::from_iter(iter);
        Self(v)
    }
}

#[cfg(test)]
mod tests {
    use crate::{from_slice, to_vec};

    use super::Array;

    #[test]
    fn test_serialize_and_deserialize_multiple_elem_array() {
        let expected = Array(vec![1i32, 2, 3]);
        let buf = to_vec(&expected).unwrap();
        let array: Array<i32> = from_slice(&buf).unwrap();
        assert_eq!(array, expected);

        let expected = Array(vec![
            String::from("abc"),
            String::from("def"),
            String::from("ghi"),
        ]);
        let buf = to_vec(&expected).unwrap();
        let array: Array<String> = from_slice(&buf).unwrap();
        assert_eq!(array, expected);
    }

    #[test]
    fn test_serialize_and_deserialize_array_encoded_single_elem_array() {
        let expected = Array(vec![1i32]);
        let buf = to_vec(&expected).unwrap();
        let array: Array<i32> = from_slice(&buf).unwrap();
        assert_eq!(array, expected);

        let expected = Array(vec![String::from("abc")]);
        let buf = to_vec(&expected).unwrap();
        let array: Array<String> = from_slice(&buf).unwrap();
        assert_eq!(array, expected);
    }

    #[test]
    fn test_serialize_and_deserialize_single_value_encoded_single_elem_array() {
        let value = 1i32;
        let expected = Array(vec![value]);
        let buf = to_vec(&value).unwrap();
        let array: Array<i32> = from_slice(&buf).unwrap();
        assert_eq!(array, expected);

        let value = String::from("abc");
        let expected = Array(vec![value.clone()]);
        let buf = to_vec(&value).unwrap();
        let array: Array<String> = from_slice(&buf).unwrap();
        assert_eq!(array, expected);
    }
}
