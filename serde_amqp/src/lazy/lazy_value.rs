use std::ops::Deref;

use bytes::Bytes;
use serde::de;

use crate::{format::Category, format_code::EncodingCodes, read::Read, Error};

/// A lazy value
#[derive(Debug, Clone)]
pub struct LazyValue(Bytes);

impl LazyValue {
    /// Create a new LazyValue
    pub fn from_reader<'de, R>(mut reader: R) -> Result<Self, Error>
    where
        R: Read<'de>,
    {
        let code: EncodingCodes = reader
            .peek()
            .ok_or(Error::unexpected_eof("parse LazyValue"))
            .and_then(|code| code.try_into())?;


        let bytes = match Category::try_from(code) {
            Ok(Category::Fixed(width)) => read_fixed(reader, width)?,
            Ok(Category::Variable(width)) => read_encoded_len(reader, width)?,
            Ok(Category::Compound(width)) => read_encoded_len(reader, width)?,
            Ok(Category::Array(width)) => read_encoded_len(reader, width)?,
            Err(_is_described) => read_described(reader)?,
        };

        Ok(LazyValue(Bytes::from(bytes)))
    }

    /// Get the bytes as a slice
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }
}

fn read_fixed<'de, R>(mut reader: R, width: usize) -> Result<Vec<u8>, Error> 
where 
    R: Read<'de>,
{
    let bytes = reader.read_bytes(width + 1)?;
    Ok(bytes)
}

fn read_encoded_len<'de, R>(mut reader: R, width: usize) -> Result<Vec<u8>, Error> 
where 
    R: Read<'de>,
{
    let len_bytes = reader.peek_bytes(width + 1)?
        .ok_or_else(|| Error::unexpected_eof("parse LazyValue"))?;

    match width {
        1 => {
            let len = len_bytes[1] as usize;
            let bytes = reader.read_bytes(1 + width + len)?;
            Ok(bytes)
        }
        4 => {
            let mut len_bytes_ = [0; 4];
            len_bytes_.copy_from_slice(&len_bytes[1..]);
            let len = u32::from_be_bytes(len_bytes_) as usize;
            let bytes = reader.read_bytes(1 + width + len)?;
            Ok(bytes)
        }
        _ => unreachable!(),
    }
}

fn read_described<'de, R>(mut reader: R) -> Result<Vec<u8>, Error> 
where 
    R: Read<'de>,
{
    todo!()
}

#[cfg(test)]
mod tests {
    use crate::{primitives::Array, to_vec};

    use super::*;

    #[test]
    fn test_read_fixed() {
        let bytes = [
            EncodingCodes::Ubyte as u8,
            0x01,
        ];

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
}