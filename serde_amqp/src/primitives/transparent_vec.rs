//! Implement transparent vec

use std::{marker::PhantomData, ops::{Deref, DerefMut}};

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

impl<T> From<Vec<T>> for TransparentVec<T> {
    fn from(inner: Vec<T>) -> Self {
        Self(inner)
    }
}

impl<T> From<TransparentVec<T>> for Vec<T> {
    fn from(val: TransparentVec<T>) -> Self {
        val.0
    }
}

impl<T> AsMut<Vec<T>> for TransparentVec<T> {
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T> Deref for TransparentVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for TransparentVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}


impl<T> IntoIterator for TransparentVec<T> {
    type Item = T;

    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a TransparentVec<T> {
    type Item = &'a T;

    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut TransparentVec<T> {
    type Item = &'a mut T;

    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<T> FromIterator<T> for TransparentVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let v = Vec::from_iter(iter);
        Self(v)
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
    use serde_amqp_derive::{SerializeComposite, DeserializeComposite};

    use crate::{format_code::EncodingCodes, from_slice, to_vec};

    use super::TransparentVec;

    #[test]
    fn test_serialize_primitive_transparent_vec() {
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
    fn test_deserialize_primitive_transparent_vec() {
        let buf = vec![
            EncodingCodes::BooleanTrue as u8,
            EncodingCodes::BooleanFalse as u8,
            EncodingCodes::BooleanTrue as u8,
        ];
        let vec: TransparentVec<bool> = from_slice(&buf).unwrap();
        let expected = TransparentVec::new(vec![true, false, true]);
        assert_eq!(vec, expected);
    }

    use crate as serde_amqp;

    #[derive(Debug, Clone, SerializeComposite, DeserializeComposite, PartialEq, PartialOrd)]
    #[amqp_contract(
        code = 0x13,
        name = "test:example",
        encoding = "list",
        rename_all = "kebab-case"
    )]
    struct Foo {
        field_num: u32,
        field_bool: bool,
    }

    #[test]
    fn test_serialize_composite_transparent_vec() {
        let foo1 = Foo { field_num: 9, field_bool: true };
        let foo2 = Foo { field_num: 63, field_bool: false };
        let foo3 = Foo { field_num: 88, field_bool: true };
        let vec = TransparentVec::new(vec![foo1.clone(), foo2.clone(), foo3.clone()]);
        let buf = to_vec(&vec).unwrap();

        let mut expected = Vec::new();
        expected.extend(to_vec(&foo1).unwrap());
        expected.extend(to_vec(&foo2).unwrap());
        expected.extend(to_vec(&foo3).unwrap());

        assert_eq!(buf, expected);
    }

    #[test]
    fn test_deserialize_composite_transparent_vec() {
        let foo1 = Foo { field_num: 9, field_bool: true };
        let foo2 = Foo { field_num: 63, field_bool: false };
        let foo3 = Foo { field_num: 88, field_bool: true };

        let mut buf = Vec::new();
        buf.extend(to_vec(&foo1).unwrap());
        buf.extend(to_vec(&foo2).unwrap());
        buf.extend(to_vec(&foo3).unwrap());
        let vec: TransparentVec<Foo> = from_slice(&buf).unwrap();
        let expected = TransparentVec::new(vec![foo1, foo2, foo3]);
        assert_eq!(vec, expected);
    }
}
