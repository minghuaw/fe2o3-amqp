//! Implements `LazyValue`

use bytes::Bytes;
use serde::{de::Visitor, Deserialize, Serialize};

use crate::{
    read::{read_described_bytes, read_primitive_bytes_or_else, Read, SliceReader},
    Error,
    __constants::LAZY_VALUE,
};

/// A lazy value
///
/// This unfortunately does not implement `serde::Deserialize` or `serde::Serialize` yet. Please
/// use [`LazyValue::from_reader`] to decode from a reader.
#[derive(Debug, Clone)]
pub struct LazyValue(Bytes);

impl LazyValue {
    /// Create a new LazyValue
    pub fn from_reader<'de, R>(reader: &mut R) -> Result<Self, Error>
    where
        R: Read<'de>,
    {
        let bytes = read_primitive_bytes_or_else(reader, read_described_bytes)?;
        Ok(LazyValue(Bytes::from(bytes)))
    }

    /// Get the bytes as a slice
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    /// Get the underlying bytes
    pub fn as_bytes(&self) -> &Bytes {
        &self.0
    }

    /// Convert the lazy value into bytes
    pub fn into_bytes(self) -> Bytes {
        self.0
    }
}

impl From<LazyValue> for Bytes {
    fn from(lazy_value: LazyValue) -> Self {
        lazy_value.into_bytes()
    }
}

impl From<LazyValue> for Vec<u8> {
    fn from(lazy_value: LazyValue) -> Self {
        lazy_value.into_bytes().to_vec()
    }
}

impl Serialize for LazyValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct(LAZY_VALUE, serde_bytes::Bytes::new(self.as_slice()))
    }
}

struct LazyValueVisitor;

impl<'de> Visitor<'de> for LazyValueVisitor {
    type Value = LazyValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("LazyValue")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut reader = SliceReader::new(v);
        LazyValue::from_reader(&mut reader).map_err(serde::de::Error::custom)
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut reader = SliceReader::new(v);
        LazyValue::from_reader(&mut reader).map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for LazyValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(LAZY_VALUE, LazyValueVisitor)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        described::Described, descriptor::Descriptor, format_code::EncodingCodes, from_slice,
        primitives::Array, to_vec,
    };

    use super::*;

    #[test]
    fn test_read_fixed_bytes() {
        let bytes = [EncodingCodes::Ubyte as u8, 0x01];

        let mut reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(&mut reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);
    }

    #[test]
    fn test_read_variable() {
        let src = "Hello, World!";
        let bytes = to_vec(&src).unwrap();
        let mut reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(&mut reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);
    }

    #[test]
    fn test_read_compound() {
        let src = [1, 2, 3, 4, 5];
        let bytes = to_vec(&src).unwrap();
        let mut reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(&mut reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);
    }

    #[test]
    fn test_read_array() {
        let src = Array(vec![1, 2, 3, 4, 5]);
        let bytes = to_vec(&src).unwrap();
        let mut reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(&mut reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);
    }

    #[test]
    fn test_read_described() {
        // Described with a code descriptor
        let descriptor = Descriptor::Code(0x13);
        let value = vec![1, 2, 3, 4, 5];
        let described = Described { descriptor, value };

        let bytes = to_vec(&described).unwrap();
        let mut reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(&mut reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);

        // Described with a name descriptor
        let descriptor = Descriptor::Name("test:name".into());
        let value = vec![1, 2, 3, 4, 5];
        let described = Described { descriptor, value };

        let bytes = to_vec(&described).unwrap();
        let mut reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(&mut reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);
    }

    #[test]
    fn test_serialize() {
        let src = "Hello, World!";
        let bytes = to_vec(&src).unwrap();
        let mut reader = SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(&mut reader).unwrap();

        assert_eq!(lazy_value.as_slice(), &bytes);

        let serialized = to_vec(&lazy_value).unwrap();
        assert_eq!(serialized, bytes);
    }

    #[test]
    fn test_serialize_compound() {
        let src_str = "Hello, World!";
        let bytes = to_vec(&src_str).unwrap();
        let mut reader = SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(&mut reader).unwrap();

        let list = [&lazy_value, &lazy_value, &lazy_value];
        let serialized = to_vec(&list).unwrap();

        let expected_list = [&src_str, &src_str, &src_str];
        let expected_bytes = to_vec(&expected_list).unwrap();

        assert_eq!(serialized, expected_bytes);
    }

    #[test]
    fn test_deserialize() {
        let src = "Hello, World!";
        let bytes = to_vec(&src).unwrap();
        let lazy: LazyValue = from_slice(&bytes).unwrap();

        assert_eq!(lazy.as_slice(), &bytes);
    }

    #[test]
    fn test_deserialize_compound() {
        let src_str = "Hello, World!";
        let list = [&src_str, &src_str, &src_str];
        let bytes = to_vec(&list).unwrap();

        let lazy: Vec<LazyValue> = from_slice(&bytes).unwrap();
        let expected_bytes = to_vec(&src_str).unwrap();

        for lazy_value in lazy {
            assert_eq!(lazy_value.as_slice(), &expected_bytes);
        }
    }
}
