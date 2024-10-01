//! Implements `LazyValue`

use bytes::Bytes;

use crate::{
    read::{read_described_bytes, read_primitive_bytes_or_else, Read},
    Error,
};

/// A lazy value
///
/// This unfortunately does not implement `serde::Deserialize` or `serde::Serialize` yet. Please
/// use [`LazyValue::from_reader`] to decode from a reader.
#[derive(Debug, Clone)]
pub struct LazyValue(Bytes);

impl LazyValue {
    /// Create a new LazyValue
    pub fn from_reader<'de, R>(reader: R) -> Result<Self, Error>
    where
        R: Read<'de>,
    {
        let (bytes, _) = read_primitive_bytes_or_else(reader, read_described_bytes)?;
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

#[cfg(test)]
mod tests {
    use crate::{
        described::Described, descriptor::Descriptor, format_code::EncodingCodes,
        primitives::Array, to_vec,
    };

    use super::*;

    #[test]
    fn test_read_fixed_bytes() {
        let bytes = [EncodingCodes::Ubyte as u8, 0x01];

        let reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);
    }

    #[test]
    fn test_read_variable() {
        let src = "Hello, World!";
        let bytes = to_vec(&src).unwrap();
        let reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);
    }

    #[test]
    fn test_read_compound() {
        let src = [1, 2, 3, 4, 5];
        let bytes = to_vec(&src).unwrap();
        let reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);
    }

    #[test]
    fn test_read_array() {
        let src = Array(vec![1, 2, 3, 4, 5]);
        let bytes = to_vec(&src).unwrap();
        let reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);
    }

    #[test]
    fn test_read_described() {
        // Described with a code descriptor
        let descriptor = Descriptor::Code(0x13);
        let value = vec![1, 2, 3, 4, 5];
        let described = Described { descriptor, value };

        let bytes = to_vec(&described).unwrap();
        let reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);

        // Described with a name descriptor
        let descriptor = Descriptor::Name("test:name".into());
        let value = vec![1, 2, 3, 4, 5];
        let described = Described { descriptor, value };

        let bytes = to_vec(&described).unwrap();
        let reader = crate::read::SliceReader::new(&bytes);
        let lazy_value = LazyValue::from_reader(reader).unwrap();
        assert_eq!(lazy_value.as_slice(), &bytes);
    }
}
