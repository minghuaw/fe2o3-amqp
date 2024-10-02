//! Implements `LazyValue`

use bytes::Bytes;
use serde::{de::Visitor, Deserialize, Serialize};

use crate::{
    read::{read_described_bytes, read_primitive_bytes_or_else, Read},
    Error,
    __constants::LAZY_VALUE,
};

/// A lazy value.
///
/// This is a thin wrapper around a [`Bytes`] value that can be accessed via
/// [`LazyValue::as_slice`], [`LazyValue::as_bytes`], or [`LazyValue::into_bytes`], which
/// can then be used for lazy deserialization.
///
/// An empty message body may be represented as a `LazyValue` with a Null byte. This is to
/// ensure that it can be serialized/deserialized into a valid AMQP value.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LazyValue(Bytes);

impl LazyValue {
    /// Reads the bytes of the next valid AMQP value from the reader and returns a [`LazyValue`]
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

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(LazyValue(Bytes::from(v)))
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
        from_value, primitives::Array, read::SliceReader, to_value, to_vec, Value,
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
    fn test_serialize_into_value() {
        let src = "Hello, World!";
        let bytes = to_vec(&src).unwrap();
        let mut reader = SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(&mut reader).unwrap();

        let value = to_value(&lazy_value).unwrap();
        assert_eq!(value, Value::String(src.into()));
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

    #[test]
    fn test_deserialize_from_value() {
        let src = "Hello, World!";
        let value = Value::String(src.into());
        let lazy: LazyValue = from_value(value).unwrap();

        let expected = to_vec(&src).unwrap();
        assert_eq!(lazy.as_slice(), &expected);
    }
}
