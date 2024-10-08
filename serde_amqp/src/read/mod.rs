//! Custom `Read` trait

use std::io;

use crate::{error::Error, format::Category, format_code::EncodingCodes};

mod ioread;
pub use ioread::*;

mod sliceread;
pub use sliceread::*;

mod private {
    pub trait Sealed {}
}

/// A custom Read trait for internal use
pub trait Read<'de>: private::Sealed {
    /// Peek the next byte without consuming
    fn peek(&mut self) -> Option<u8>;

    /// Read the next byte
    fn next(&mut self) -> Result<Option<u8>, io::Error>;

    /// Read n bytes
    ///
    /// Prefered to use this when the size is small and can be stack allocated
    fn read_const_bytes<const N: usize>(&mut self) -> Result<[u8; N], io::Error> {
        let mut buf = [0u8; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Peek `n` number of bytes without consuming
    fn peek_bytes(&mut self, n: usize) -> Result<Option<&[u8]>, io::Error>;

    /// Consuming `n` number of bytes
    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = vec![0u8; n];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Read to buffer
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), io::Error>;

    /// Forward bytes to visitor
    fn forward_read_bytes_with_hint<V>(
        &mut self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>;

    /// Forward bytes to visitor without hint. The reader would attempt to read the bytes
    /// of the next value into a buffer and then forward it to the visitor.
    fn forward_read_byte_buf<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>;

    /// Forward str to visitor
    fn forward_read_str<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>;
}

fn read_fixed_bytes<'de, R>(reader: &mut R, width: usize) -> Result<Vec<u8>, Error>
where
    R: Read<'de>,
{
    let bytes = reader.read_bytes(width + 1)?;
    Ok(bytes)
}

fn peek_encoded_len<'de, R>(reader: &mut R, width: usize) -> Result<usize, Error>
where
    R: Read<'de>,
{
    let len_bytes = reader
        .peek_bytes(width + 1)?
        .ok_or_else(|| Error::unexpected_eof("parse LazyValue"))?;
    match width {
        1 => {
            let len = len_bytes[1] as usize;
            Ok(len)
        }
        4 => {
            let mut len_bytes_ = [0; 4];
            len_bytes_.copy_from_slice(&len_bytes[1..]);
            let len = u32::from_be_bytes(len_bytes_) as usize;
            Ok(len)
        }
        _ => unreachable!(),
    }
}

fn read_encoded_len_bytes<'de, R>(reader: &mut R, width: usize) -> Result<Vec<u8>, Error>
where
    R: Read<'de>,
{
    let len = peek_encoded_len(reader, width)?;
    let bytes = reader.read_bytes(1 + width + len)?;
    Ok(bytes)
}

/// Read bytes of a primitive type or calls `op` if the encoded type is a described type
pub(crate) fn read_primitive_bytes_or_else<'de, R, F>(
    reader: &mut R,
    op: F,
) -> Result<Vec<u8>, Error>
where
    R: Read<'de>,
    F: FnOnce(&mut R) -> Result<Vec<u8>, Error>,
{
    let code: EncodingCodes = reader
        .peek()
        .ok_or(Error::unexpected_eof("parse LazyValue"))
        .and_then(|code| code.try_into())?;

    let bytes = match Category::try_from(code) {
        Ok(Category::Fixed(width)) => read_fixed_bytes(reader, width)?,
        Ok(Category::Variable(width)) => read_encoded_len_bytes(reader, width)?,
        Ok(Category::Compound(width)) => read_encoded_len_bytes(reader, width)?,
        Ok(Category::Array(width)) => read_encoded_len_bytes(reader, width)?,
        Err(_is_described) => op(reader)?,
    };

    Ok(bytes)
}

/// Read bytes of a described type
pub(crate) fn read_described_bytes<'de, R>(reader: &mut R) -> Result<Vec<u8>, Error>
where
    R: Read<'de>,
{
    // Read 0x00
    let mut bytes = reader.read_bytes(1)?;

    // Read the descriptor
    let mut descriptor_bytes =
        read_primitive_bytes_or_else(reader, |_| Err(Error::InvalidFormatCode))?;
    bytes.append(&mut descriptor_bytes);

    // Read the value
    let mut value_bytes = read_primitive_bytes_or_else(reader, |_| Err(Error::InvalidFormatCode))?;
    bytes.append(&mut value_bytes);

    Ok(bytes)
}
