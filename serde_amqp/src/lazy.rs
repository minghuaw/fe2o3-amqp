//! Implements `LazyValue`

use bytes::Bytes;

use crate::{format::Category, format_code::EncodingCodes, read::Read, Error};

fn read_fixed_bytes<'de, R>(mut reader: R, width: usize) -> Result<(Vec<u8>, R), Error>
where
    R: Read<'de>,
{
    let bytes = reader.read_bytes(width + 1)?;
    Ok((bytes, reader))
}

fn read_encoded_len<'de, R>(mut reader: R, width: usize) -> Result<(usize, R), Error>
where
    R: Read<'de>,
{
    let len_bytes = reader
        .peek_bytes(width + 1)?
        .ok_or_else(|| Error::unexpected_eof("parse LazyValue"))?;
    match width {
        1 => {
            let len = len_bytes[1] as usize;
            Ok((len, reader))
        }
        4 => {
            let mut len_bytes_ = [0; 4];
            len_bytes_.copy_from_slice(&len_bytes[1..]);
            let len = u32::from_be_bytes(len_bytes_) as usize;
            Ok((len, reader))
        }
        _ => unreachable!(),
    }
}

fn read_encoded_len_bytes<'de, R>(reader: R, width: usize) -> Result<(Vec<u8>, R), Error>
where
    R: Read<'de>,
{
    let (len, mut reader) = read_encoded_len(reader, width)?;
    let bytes = reader.read_bytes(1 + width + len)?;
    Ok((bytes, reader))
}

fn read_primitive_or_else<'de, R, F>(mut reader: R, f: F) -> Result<(Vec<u8>, R), Error>
where
    R: Read<'de>,
    F: FnOnce(R) -> Result<(Vec<u8>, R), Error>,
{
    let code: EncodingCodes = reader
        .peek()
        .ok_or(Error::unexpected_eof("parse LazyValue"))
        .and_then(|code| code.try_into())?;

    let (bytes, reader) = match Category::try_from(code) {
        Ok(Category::Fixed(width)) => read_fixed_bytes(reader, width)?,
        Ok(Category::Variable(width)) => read_encoded_len_bytes(reader, width)?,
        Ok(Category::Compound(width)) => read_encoded_len_bytes(reader, width)?,
        Ok(Category::Array(width)) => read_encoded_len_bytes(reader, width)?,
        Err(_is_described) => f(reader)?,
    };

    Ok((bytes, reader))
}

fn read_described<'de, R>(mut reader: R) -> Result<(Vec<u8>, R), Error>
where
    R: Read<'de>,
{
    // Read 0x00
    let mut bytes = reader.read_bytes(1)?;

    // Read the descriptor
    let (mut descriptor_bytes, reader) =
        read_primitive_or_else(reader, |_| Err(Error::InvalidFormatCode))?;
    bytes.append(&mut descriptor_bytes);

    // Read the value
    let (mut value_bytes, reader) =
        read_primitive_or_else(reader, |_| Err(Error::InvalidFormatCode))?;
    bytes.append(&mut value_bytes);

    Ok((bytes, reader))
}

/// A lazy value
#[derive(Debug, Clone)]
pub struct LazyValue(Bytes);

impl LazyValue {
    /// Create a new LazyValue
    pub fn from_reader<'de, R>(reader: R) -> Result<Self, Error>
    where
        R: Read<'de>,
    {
        let (bytes, _) = read_primitive_or_else(reader, read_described)?;
        Ok(LazyValue(Bytes::from(bytes)))
    }

    /// Get the bytes as a slice
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use crate::{described::Described, descriptor::Descriptor, primitives::Array, to_vec};

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
