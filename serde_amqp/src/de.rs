//! Deserializer implementation

use serde::{
    de::{self},
    Deserialize,
};
use std::convert::TryInto;

use crate::{
    __constants::{
        ARRAY, DECIMAL128, DECIMAL32, DECIMAL64, DESCRIBED_BASIC, DESCRIBED_LIST, DESCRIBED_MAP,
        DESCRIPTOR, SYMBOL, SYMBOL_REF, TIMESTAMP, TRANSPARENT_VEC, UUID, VALUE,
    },
    descriptor::PeekDescriptor,
    error::Error,
    fixed_width::{DECIMAL128_WIDTH, DECIMAL32_WIDTH, DECIMAL64_WIDTH, UUID_WIDTH},
    format::{
        OFFSET_ARRAY32, OFFSET_ARRAY8, OFFSET_LIST32, OFFSET_LIST8, OFFSET_MAP32, OFFSET_MAP8,
    },
    format_code::EncodingCodes,
    read::{IoReader, Read, SliceReader},
    util::{EnumType, NewType, PeekTypeCode, StructEncoding},
};

/// Deserialize an instance of type T from an IO stream
pub fn from_reader<T: de::DeserializeOwned>(reader: impl std::io::Read) -> Result<T, Error> {
    let reader = IoReader::new(reader);
    let mut de = Deserializer::new(reader);
    T::deserialize(&mut de)
}

/// Deserialize and instance of type T from a bytes slice
pub fn from_slice<'de, T: de::Deserialize<'de>>(slice: &'de [u8]) -> Result<T, Error> {
    let reader = SliceReader::new(slice);
    let mut de = Deserializer::new(reader);
    T::deserialize(&mut de)
}

/// A structure that deserializes AMQP1.0 binary encoded values into rust types
#[derive(Debug)]
pub struct Deserializer<R> {
    reader: R,
    new_type: NewType,
    enum_type: EnumType,
    struct_encoding: StructEncoding,
    elem_format_code: Option<EncodingCodes>,
}

impl<'de, R: Read<'de>> Deserializer<R> {
    /// Creates a new AMQP1.0 (crate)deserializer
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            new_type: Default::default(),
            enum_type: Default::default(),
            struct_encoding: StructEncoding::None,
            elem_format_code: None,
        }
    }

    fn read_format_code(&mut self) -> Option<Result<EncodingCodes, Error>> {
        let code = self.reader.next();
        let code = code?;
        Some(code.try_into())
    }

    fn get_elem_code_or_read_format_code(&mut self) -> Option<Result<EncodingCodes, Error>> {
        match &self.elem_format_code {
            Some(c) => Some(Ok(c.clone())),
            None => self.read_format_code(),
        }
    }

    fn get_elem_code_or_peek_byte(&mut self) -> Option<Result<u8, Error>> {
        match &self.elem_format_code {
            Some(c) => Some(Ok(c.clone() as u8)),
            None => self.reader.peek().map(Ok),
        }
    }

    #[inline]
    fn parse_bool(&mut self) -> Result<bool, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_bool"))??
        {
            EncodingCodes::Boolean => {
                let byte = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof("Expecting bool byte"))?;
                match byte {
                    0x00 => Ok(false),
                    0x01 => Ok(true),
                    _ => Err(Error::InvalidValue),
                }
            }
            EncodingCodes::BooleanTrue => Ok(true),
            EncodingCodes::BooleanFalse => Ok(false),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_i8(&mut self) -> Result<i8, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_i8"))??
        {
            EncodingCodes::Byte => {
                let byte = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof("Expecting i8"))?;
                Ok(byte as i8)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_i16(&mut self) -> Result<i16, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_i16"))??
        {
            EncodingCodes::Short => {
                let bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof("Expecting i16"))?;
                Ok(i16::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_i32(&mut self) -> Result<i32, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_i32"))??
        {
            EncodingCodes::Int => {
                let bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof("Expecting i32"))?;
                Ok(i32::from_be_bytes(bytes))
            }
            EncodingCodes::SmallInt => {
                let signed = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))? as i8;
                Ok(signed as i32)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_i64(&mut self) -> Result<i64, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_i64"))??
        {
            EncodingCodes::Long => {
                let bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof("Expecting i64"))?;
                Ok(i64::from_be_bytes(bytes))
            }
            EncodingCodes::SmallLong => {
                let signed = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))? as i8;
                Ok(signed as i64)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_u8(&mut self) -> Result<u8, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_u8"))??
        {
            EncodingCodes::UByte => {
                let byte = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof("Expecting u8"))?;
                Ok(byte)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_u16(&mut self) -> Result<u16, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_u16"))??
        {
            EncodingCodes::UShort => {
                let bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof("Expecting u16"))?;
                Ok(u16::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_u32(&mut self) -> Result<u32, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_u32"))??
        {
            EncodingCodes::UInt => {
                let bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof("Expecting u32"))?;
                Ok(u32::from_be_bytes(bytes))
            }
            EncodingCodes::SmallUInt => {
                let byte = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof("Expecting small u32"))?;
                Ok(byte as u32)
            }
            EncodingCodes::UInt0 => Ok(0),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_u64(&mut self) -> Result<u64, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_u64"))??
        {
            EncodingCodes::ULong => {
                let bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof("Expecting u64"))?;
                Ok(u64::from_be_bytes(bytes))
            }
            EncodingCodes::SmallULong => {
                let byte = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof("Expecting small u64"))?;
                Ok(byte as u64)
            }
            EncodingCodes::ULong0 => Ok(0),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_f32(&mut self) -> Result<f32, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_f32"))??
        {
            EncodingCodes::Float => {
                let bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof("Expecting f32"))?;
                Ok(f32::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_f64(&mut self) -> Result<f64, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_f64"))??
        {
            EncodingCodes::Double => {
                let bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof("Expecting f64"))?;
                Ok(f64::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_char(&mut self) -> Result<char, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_char"))??
        {
            EncodingCodes::Char => {
                let bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof("Expecting char"))?;
                let n = u32::from_be_bytes(bytes);
                char::from_u32(n).ok_or(Error::InvalidValue)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn read_small_string(&mut self) -> Option<Result<String, Error>> {
        let len = self.reader.next()?;
        let buf = self.reader.read_bytes(len as usize)?;
        Some(String::from_utf8(buf).map_err(Into::into))
    }

    #[inline]
    fn read_string(&mut self) -> Option<Result<String, Error>> {
        let len_bytes = self.reader.read_const_bytes()?;
        let len = u32::from_be_bytes(len_bytes);
        let buf = self.reader.read_bytes(len as usize)?;
        Some(String::from_utf8(buf).map_err(Into::into))
    }

    #[inline]
    fn parse_string(&mut self) -> Result<String, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_string"))??
        {
            EncodingCodes::Str8 => self
                .read_small_string()
                .ok_or_else(|| Error::unexpected_eof("Expecting str8"))?,
            EncodingCodes::Str32 => self
                .read_string()
                .ok_or_else(|| Error::unexpected_eof("Expecting str32"))?,
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_symbol(&mut self) -> Result<String, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_symbol"))??
        {
            EncodingCodes::Sym8 => self
                .read_small_string()
                .ok_or_else(|| Error::unexpected_eof("Expecting sym8"))?,
            EncodingCodes::Sym32 => self
                .read_string()
                .ok_or_else(|| Error::unexpected_eof("Expecting sym32"))?,
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_byte_buf(&mut self) -> Result<Vec<u8>, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_byte_buf"))??
        {
            EncodingCodes::VBin8 => {
                let len = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof("Expecting len"))?;
                self.reader
                    .read_bytes(len as usize)
                    .ok_or_else(|| Error::unexpected_eof("Expecting vbin8"))
            }
            EncodingCodes::VBin32 => {
                let len_bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof("Expecting len"))?;
                let len = u32::from_be_bytes(len_bytes);
                self.reader
                    .read_bytes(len as usize)
                    .ok_or_else(|| Error::unexpected_eof("Expecting vbin32"))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_decimal<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_decimal"))??
        {
            EncodingCodes::Decimal32 => self.reader.forward_read_bytes(DECIMAL32_WIDTH, visitor),
            EncodingCodes::Decimal64 => self.reader.forward_read_bytes(DECIMAL64_WIDTH, visitor),
            EncodingCodes::Decimal128 => self.reader.forward_read_bytes(DECIMAL128_WIDTH, visitor),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_uuid<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_uuid"))??
        {
            EncodingCodes::Uuid => self.reader.forward_read_bytes(UUID_WIDTH, visitor),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_timestamp(&mut self) -> Result<i64, Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_timestamp"))??
        {
            EncodingCodes::Timestamp => {
                let bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof("Expecting timestamp"))?;
                Ok(i64::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_unit(&mut self) -> Result<(), Error> {
        match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("parse_unit"))??
        {
            EncodingCodes::Null => Ok(()),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_described_identifier<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: de::Visitor<'de>,
    {
        // [0] is 0x00,
        // [1] is format code
        let buf = self
            .reader
            .peek_bytes(2)
            .ok_or_else(|| Error::unexpected_eof("parse_described_identifier"))?;
        let code = buf[1];
        match code.try_into()? {
            EncodingCodes::Sym8 => {
                // [0] is 0x00,
                // [1] is format code
                // [2] is size
                let _buf = self
                    .reader
                    .peek_bytes(3)
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let size = _buf[2] as usize;
                let _buf = self
                    .reader
                    .peek_bytes(3 + size)
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let slice = std::str::from_utf8(&_buf[3..])?;
                visitor.visit_str(slice)
            }
            EncodingCodes::Sym32 => {
                // [0] is 0x00,
                // [1] is format code
                // [2..6] are size
                let _buf = self
                    .reader
                    .peek_bytes(2 + 4)
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let mut size_bytes = [0u8; 4];
                size_bytes.copy_from_slice(&_buf[2..]);
                let size = u32::from_be_bytes(size_bytes) as usize;
                let _buf = self
                    .reader
                    .peek_bytes(6 + size)
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let slice = std::str::from_utf8(&_buf[6..])?;
                visitor.visit_str(slice)
            }
            EncodingCodes::ULong0 => visitor.visit_u64(0),
            EncodingCodes::SmallULong => {
                // [0] is 0x00,
                // [1] is format code
                // [2] is the value
                let buf = self
                    .reader
                    .peek_bytes(3)
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let value = buf[2];
                visitor.visit_u64(value as u64)
            }
            EncodingCodes::ULong => {
                // [0] is 0x00,
                // [1] is format code
                // [2..10] is value bytes
                let slice = self
                    .reader
                    .peek_bytes(2 + 8)
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&slice[2..]);
                let value = u64::from_be_bytes(bytes);
                visitor.visit_u64(value)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }
}

impl<'de, 'a, R> de::Deserializer<'de> for &'a mut Deserializer<R>
where
    R: Read<'de>,
{
    type Error = Error;

    fn is_human_readable(&self) -> bool {
        false
    }

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // match self.reader.peek()?.try_into()? {
        match self
            .get_elem_code_or_peek_byte()
            .ok_or_else(|| Error::unexpected_eof(""))??
            .try_into()?
        {
            EncodingCodes::Boolean | EncodingCodes::BooleanFalse | EncodingCodes::BooleanTrue => {
                self.deserialize_bool(visitor)
            }
            EncodingCodes::Byte => self.deserialize_i8(visitor),
            EncodingCodes::Short => self.deserialize_i16(visitor),
            EncodingCodes::Int | EncodingCodes::SmallInt => self.deserialize_i32(visitor),
            EncodingCodes::Long | EncodingCodes::SmallLong => self.deserialize_i64(visitor),
            EncodingCodes::UByte => self.deserialize_u8(visitor),
            EncodingCodes::UShort => self.deserialize_u16(visitor),
            EncodingCodes::UInt | EncodingCodes::SmallUInt | EncodingCodes::UInt0 => {
                self.deserialize_u32(visitor)
            }
            EncodingCodes::ULong | EncodingCodes::SmallULong | EncodingCodes::ULong0 => {
                self.deserialize_u64(visitor)
            }
            EncodingCodes::Float => self.deserialize_f32(visitor),
            EncodingCodes::Double => self.deserialize_f64(visitor),
            EncodingCodes::Char => self.deserialize_char(visitor),
            EncodingCodes::Str32 | EncodingCodes::Str8 => self.deserialize_string(visitor),
            EncodingCodes::VBin32 | EncodingCodes::VBin8 => self.deserialize_byte_buf(visitor),
            EncodingCodes::Null => self.deserialize_unit(visitor),

            EncodingCodes::Sym32 | EncodingCodes::Sym8 => {
                self.deserialize_newtype_struct(SYMBOL, visitor)
            }
            EncodingCodes::DescribedType => {
                // This will not handle DescribedBasic types
                self.deserialize_struct("", &[""], visitor)
            }
            EncodingCodes::Array32 | EncodingCodes::Array8 => {
                self.deserialize_newtype_struct(ARRAY, visitor)
            }
            EncodingCodes::List0 | EncodingCodes::List8 | EncodingCodes::List32 => {
                self.deserialize_seq(visitor)
            }
            EncodingCodes::Map32 | EncodingCodes::Map8 => self.deserialize_map(visitor),

            EncodingCodes::Decimal32 => self.deserialize_newtype_struct(DECIMAL32, visitor),
            EncodingCodes::Decimal64 => self.deserialize_newtype_struct(DECIMAL64, visitor),
            EncodingCodes::Decimal128 => self.deserialize_newtype_struct(DECIMAL128, visitor),
            EncodingCodes::Timestamp => self.deserialize_newtype_struct(TIMESTAMP, visitor),
            EncodingCodes::Uuid => self.deserialize_newtype_struct(UUID, visitor),
        }
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    #[inline]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i8(self.parse_i8()?)
    }

    #[inline]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i16(self.parse_i16()?)
    }

    #[inline]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i32(self.parse_i32()?)
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.new_type {
            NewType::Timestamp => {
                self.new_type = NewType::None;
                visitor.visit_i64(self.parse_timestamp()?)
            }
            _ => visitor.visit_i64(self.parse_i64()?),
        }
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u8(self.parse_u8()?)
    }

    #[inline]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u16(self.parse_u16()?)
    }

    #[inline]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u32(self.parse_u32()?)
    }

    #[inline]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u64(self.parse_u64()?)
    }

    #[inline]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_f32(self.parse_f32()?)
    }

    #[inline]
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_f64(self.parse_f64()?)
    }

    #[inline]
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_char(self.parse_char()?)
    }

    #[inline]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.new_type {
            NewType::Symbol => {
                // Leave symbol as visit_string because serde(untagged)
                // on descriptor will visit String instead of str
                self.new_type = NewType::None;
                visitor.visit_string(self.parse_symbol()?)
            }
            _ => visitor.visit_string(self.parse_string()?),
        }
    }

    #[inline]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len = match self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??
        {
            EncodingCodes::Str8 | EncodingCodes::Sym8 => {
                self.reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))? as usize
            }
            EncodingCodes::Str32 | EncodingCodes::Sym32 => {
                let len_bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                u32::from_be_bytes(len_bytes) as usize
            }
            _ => return Err(Error::InvalidFormatCode),
        };
        self.reader.forward_read_str(len, visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_byte_buf(self.parse_byte_buf()?)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.new_type {
            // Use bytes to reduce number of memcpy
            NewType::Dec32 | NewType::Dec64 | NewType::Dec128 => {
                self.new_type = NewType::None;
                self.parse_decimal(visitor)
            }
            // Use bytes to reduce number of memcpy
            NewType::Uuid => {
                self.new_type = NewType::None;
                self.parse_uuid(visitor)
            }
            _ => {
                let len = match self
                    .get_elem_code_or_read_format_code()
                    .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??
                {
                    EncodingCodes::VBin8 => self
                        .reader
                        .next()
                        .ok_or_else(|| Error::unexpected_eof(""))?
                        as usize,
                    EncodingCodes::VBin32 => {
                        let bytes = self
                            .reader
                            .read_const_bytes()
                            .ok_or_else(|| Error::unexpected_eof(""))?;
                        u32::from_be_bytes(bytes) as usize
                    }
                    _ => return Err(Error::InvalidFormatCode),
                };
                self.reader.forward_read_bytes(len, visitor)
            }
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self
            .get_elem_code_or_peek_byte()
            .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??
            .try_into()?
        {
            EncodingCodes::Null => {
                // consume the Null byte
                let _ = self
                    .get_elem_code_or_read_format_code()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                visitor.visit_none()
            }
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.parse_unit().and_then(|_| visitor.visit_unit())
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if name == SYMBOL {
            self.new_type = NewType::Symbol;
            // Leave symbol as visit_string because serde(untagged)
            // on descriptor will visit String instead of str
            self.deserialize_string(visitor)
        } else if name == SYMBOL_REF {
            self.new_type = NewType::SymbolRef;
            self.deserialize_str(visitor)
        } else if name == DECIMAL32 {
            self.new_type = NewType::Dec32;
            self.deserialize_bytes(visitor)
        } else if name == DECIMAL64 {
            self.new_type = NewType::Dec64;
            self.deserialize_bytes(visitor)
        } else if name == DECIMAL128 {
            self.new_type = NewType::Dec128;
            self.deserialize_bytes(visitor)
        } else if name == UUID {
            self.new_type = NewType::Uuid;
            self.deserialize_bytes(visitor)
        } else if name == TIMESTAMP {
            self.new_type = NewType::Timestamp;
            self.deserialize_i64(visitor)
        } else if name == TRANSPARENT_VEC {
            self.new_type = NewType::TransparentVec;
            visitor.visit_seq(TransparentVecAccess::new(self))
        } else {
            visitor.visit_newtype_struct(self)
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let code = self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??;
        match code {
            EncodingCodes::Array8 => {
                // Read "header" bytes
                let len = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))? as usize;
                let count = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))? as usize;
                let format_code = self
                    .read_format_code()
                    .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??;
                self.elem_format_code = Some(format_code);

                // Account for offset
                let len = len - OFFSET_ARRAY8;
                // let buf = self.reader.read_bytes(len)?;

                visitor.visit_seq(ArrayAccess::new(self, len, count))
            }
            EncodingCodes::Array32 => {
                // Read "header" bytes
                let len_bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let count_bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let format_code = self
                    .read_format_code()
                    .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??;
                self.elem_format_code = Some(format_code);

                // Conversion
                let len = u32::from_be_bytes(len_bytes) as usize;
                let count = u32::from_be_bytes(count_bytes) as usize;

                // Account for offset
                let len = len - OFFSET_ARRAY32;
                // let buf = self.reader.read_bytes(len)?;

                visitor.visit_seq(ArrayAccess::new(self, len, count))
            }
            EncodingCodes::List0 => {
                let len = 0;
                let count = 0;
                visitor.visit_seq(ListAccess::new(self, len, count))
            }
            EncodingCodes::List8 => {
                let len = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))? as usize;
                let count = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))? as usize;

                // Account for offset
                let len = len - OFFSET_LIST8;

                // Make sure there is no other element format code
                self.elem_format_code = None;
                visitor.visit_seq(ListAccess::new(self, len, count))
            }
            EncodingCodes::List32 => {
                let len_bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let count_bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let len = u32::from_be_bytes(len_bytes) as usize;
                let count = u32::from_be_bytes(count_bytes) as usize;

                // Account for offset
                let len = len - OFFSET_LIST32;

                // Make sure there is no other element format code
                self.elem_format_code = None;
                visitor.visit_seq(ListAccess::new(self, len, count))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // Tuple will always be deserialized as List
        let code = self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??;

        let (size, count) = match code {
            EncodingCodes::List0 => {
                let size = 0;
                let count = 0;
                (size, count)
            }
            EncodingCodes::List8 => {
                let size = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))? as usize;
                let count = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))? as usize;

                // Account for offset
                let size = size - OFFSET_LIST8;

                // Make sure there is no other element format code
                self.elem_format_code = None;
                (size, count)
            }
            EncodingCodes::List32 => {
                let size_bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let count_bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let size = u32::from_be_bytes(size_bytes) as usize;
                let count = u32::from_be_bytes(count_bytes) as usize;

                // Account for offset
                let size = size - OFFSET_LIST32;

                // Make sure there is no other element format code
                self.elem_format_code = None;
                (size, count)
            }
            _ => return Err(Error::InvalidFormatCode),
        };

        if count != len {
            return Err(Error::SequenceLengthMismatch);
        }

        visitor.visit_seq(ListAccess::new(self, size, count))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let code = self
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??;
        let (size, count) = match code {
            EncodingCodes::Map8 => {
                let size = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))? as usize;
                let count = self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))? as usize;

                // Account for offset
                let size = size - OFFSET_MAP8;

                (size, count)
            }
            EncodingCodes::Map32 => {
                let size_bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let count_bytes = self
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;

                let size = u32::from_be_bytes(size_bytes) as usize;
                let count = u32::from_be_bytes(count_bytes) as usize;

                // Account for offset
                let size = size - OFFSET_MAP32;

                (size, count)
            }
            _ => return Err(Error::InvalidFormatCode),
        };

        // // AMQP map count includes both key and value, should be halfed
        // let count = count / 2;
        visitor.visit_map(MapAccess::new(self, size, count))
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if name == DESCRIBED_BASIC {
            self.struct_encoding = StructEncoding::DescribedBasic;
            visitor.visit_seq(DescribedAccess::basic(self, len as u32))
        } else if name == DESCRIBED_LIST {
            self.struct_encoding = StructEncoding::DescribedList;
            visitor.visit_seq(DescribedAccess::list(self))
        } else {
            match self
                .get_elem_code_or_peek_byte()
                .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??
                .try_into()?
            {
                EncodingCodes::DescribedType => visitor.visit_seq(DescribedAccess::list(self)),
                _ => self.deserialize_tuple(len, visitor),
            }
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // The name should override parent struct encoding
        let cur_encoding = self.struct_encoding.clone();
        let result = if name == DESCRIBED_BASIC {
            self.struct_encoding = StructEncoding::DescribedBasic;
            visitor.visit_seq(DescribedAccess::basic(self, fields.len() as u32))
        } else if name == DESCRIBED_LIST {
            self.struct_encoding = StructEncoding::DescribedList;
            visitor.visit_seq(DescribedAccess::list(self))
        } else if name == DESCRIBED_MAP {
            self.struct_encoding = StructEncoding::DescribedMap;
            visitor.visit_map(DescribedAccess::map(self))
        } else {
            self.struct_encoding = StructEncoding::None;
            match self
                .get_elem_code_or_peek_byte()
                .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??
                .try_into()?
            {
                EncodingCodes::List0 | EncodingCodes::List32 | EncodingCodes::List8 => {
                    self.deserialize_tuple(fields.len(), visitor)
                }
                EncodingCodes::Map32 | EncodingCodes::Map8 => self.deserialize_map(visitor),
                EncodingCodes::DescribedType => visitor.visit_seq(DescribedAccess::list(self)),
                _ => Err(Error::InvalidFormatCode),
            }
        };
        // Restore
        self.struct_encoding = cur_encoding;
        result
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        use crate::__constants::UNTAGGED_ENUM;

        let curr_enum_type = self.enum_type.clone();

        let result = if name == VALUE {
            self.enum_type = EnumType::Value;
            visitor.visit_enum(VariantAccess::new(self))
        } else if name == DESCRIPTOR {
            self.enum_type = EnumType::Descriptor;
            visitor.visit_enum(VariantAccess::new(self))
        } else if name == UNTAGGED_ENUM {
            visitor.visit_enum(VariantAccess::new(self))
        } else if name == ARRAY {
            self.enum_type = EnumType::Array;
            visitor.visit_enum(VariantAccess::new(self))
        } else {
            // Considering the following enum serialization format
            // `unit_variant` - a single u32
            // generic `newtype_variant` - List([u32, Value])
            // `tuple_variant` and `struct_variant` - List([u32, List([Value, *])])
            match self
                .get_elem_code_or_peek_byte()
                .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??
                .try_into()?
            {
                EncodingCodes::UInt | EncodingCodes::UInt0 | EncodingCodes::SmallUInt => {
                    visitor.visit_enum(VariantAccess::new(self))
                }
                EncodingCodes::List0 => Err(Error::InvalidFormatCode),
                EncodingCodes::List8 | EncodingCodes::Map8 => {
                    let _code = self
                        .reader
                        .next()
                        .ok_or_else(|| Error::unexpected_eof(""))?;
                    let _size = self
                        .reader
                        .next()
                        .ok_or_else(|| Error::unexpected_eof(""))?
                        as usize;
                    let count = self
                        .reader
                        .next()
                        .ok_or_else(|| Error::unexpected_eof(""))?
                        as usize;
                    if count != 2 {
                        return Err(Error::InvalidLength);
                    }
                    visitor.visit_enum(VariantAccess::new(self))
                }
                EncodingCodes::List32 | EncodingCodes::Map32 => {
                    let _code = self
                        .reader
                        .next()
                        .ok_or_else(|| Error::unexpected_eof(""))?;
                    let size_bytes = self
                        .reader
                        .read_const_bytes()
                        .ok_or_else(|| Error::unexpected_eof(""))?;
                    let _size = u32::from_be_bytes(size_bytes);
                    let count_bytes = self
                        .reader
                        .read_const_bytes()
                        .ok_or_else(|| Error::unexpected_eof(""))?;
                    let count = u32::from_be_bytes(count_bytes);

                    if count != 2 {
                        return Err(Error::InvalidLength);
                    }
                    visitor.visit_enum(VariantAccess::new(self))
                }
                // Symbols appears in the transport errors
                EncodingCodes::Sym32 | EncodingCodes::Sym8 => {
                    visitor.visit_enum(VariantAccess::new(self))
                }
                // for newtype variant of described type
                EncodingCodes::DescribedType => visitor.visit_enum(VariantAccess::new(self)),
                _ => visitor.visit_enum(VariantAccess::new(self)),
            }
        };

        // Revert self.enum_type
        self.enum_type = curr_enum_type;

        result
    }

    // an identifier is either a field of a struct or a variant of an eunm
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.enum_type {
            EnumType::Value => {
                let code = self
                    .get_elem_code_or_peek_byte()
                    .ok_or_else(|| Error::unexpected_eof(""))??;
                visitor.visit_u8(code)
            }
            EnumType::Descriptor => {
                // Consume the EncodingCodes::Described byte
                match self
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))?
                    .try_into()?
                {
                    EncodingCodes::DescribedType => {}
                    _ => return Err(Error::InvalidFormatCode),
                };
                // Reset the enum type
                self.enum_type = EnumType::None;
                let code = self
                    .get_elem_code_or_peek_byte()
                    .ok_or_else(|| Error::unexpected_eof(""))??;
                visitor.visit_u8(code)
            }
            EnumType::Array => {
                let code = self
                    .get_elem_code_or_peek_byte()
                    .ok_or_else(|| Error::unexpected_eof(""))??;
                visitor.visit_u8(code)
            }
            EnumType::None => {
                // The following are the possible identifiers
                let code = self
                    .get_elem_code_or_peek_byte()
                    .ok_or_else(|| Error::unexpected_eof(""))??;
                match code.try_into()? {
                    // If a struct is serialized as a map, then the fields are serialized as str
                    EncodingCodes::Str32 | EncodingCodes::Str8 => self.deserialize_str(visitor),
                    // FIXME: Enum variant currently are serialzied as list of with variant index and a list
                    EncodingCodes::UInt | EncodingCodes::SmallUInt | EncodingCodes::UInt0 => {
                        self.deserialize_u32(visitor)
                    }
                    // Potentially using `Descriptor::Name` as identifier
                    EncodingCodes::Sym32 | EncodingCodes::Sym8 => {
                        self.deserialize_newtype_struct(SYMBOL, visitor)
                    }
                    // Potentially using `Descriptor::Code` as identifier
                    EncodingCodes::ULong | EncodingCodes::SmallULong | EncodingCodes::ULong0 => {
                        self.deserialize_u64(visitor)
                    }
                    // Other types should not be used to serialize identifiers
                    EncodingCodes::DescribedType => self.parse_described_identifier(visitor),
                    // _ => Err(Error::InvalidFormatCode),
                    _ => visitor.visit_u8(code),
                }
            }
        }
    }

    // Use this to peek inside the buffer without consuming the bytes
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // The deserializer will only peek the next u8
        let code = self
            .reader
            .peek()
            .ok_or_else(|| Error::unexpected_eof(""))?;
        match code.try_into()? {
            EncodingCodes::DescribedType => self.parse_described_identifier(visitor),
            _ => visitor.visit_u8(code),
        }
    }
}

/// Accessor for array type
#[derive(Debug)]
pub struct ArrayAccess<'a, R> {
    de: &'a mut Deserializer<R>,
    _size: usize,
    count: usize,
}

impl<'a, R> ArrayAccess<'a, R> {
    pub(crate) fn new(de: &'a mut Deserializer<R>, size: usize, count: usize) -> Self {
        Self {
            de,
            _size: size,
            count,
        }
    }
}

impl<'a, R> AsMut<Deserializer<R>> for ArrayAccess<'a, R> {
    fn as_mut(&mut self) -> &mut Deserializer<R> {
        self.de
    }
}

impl<'a, 'de, R: Read<'de>> de::SeqAccess<'de> for ArrayAccess<'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.count {
            0 => {
                self.de.elem_format_code = None;
                Ok(None)
            }
            _ => {
                self.count -= 1;
                seed.deserialize(self.as_mut()).map(Some)
            }
        }
    }
}

/// Accessor for list type
#[derive(Debug)]
pub struct ListAccess<'a, R> {
    de: &'a mut Deserializer<R>,
    _size: usize,
    count: usize,
}

impl<'a, R> ListAccess<'a, R> {
    pub(crate) fn new(de: &'a mut Deserializer<R>, size: usize, count: usize) -> Self {
        Self {
            de,
            _size: size,
            count,
        }
    }
}

impl<'a, R> AsMut<Deserializer<R>> for ListAccess<'a, R> {
    fn as_mut(&mut self) -> &mut Deserializer<R> {
        self.de
    }
}

impl<'a, 'de, R: Read<'de>> de::SeqAccess<'de> for ListAccess<'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.count {
            0 => Ok(None),
            _ => {
                self.count -= 1;
                seed.deserialize(self.as_mut()).map(Some)
            }
        }
    }
}

/// Accessor for transparent vec type
#[derive(Debug)]
pub struct TransparentVecAccess<'a, R> {
    de: &'a mut Deserializer<R>,
    cached: Option<PeekTypeCode>,
}

impl<'a, R> TransparentVecAccess<'a, R> {
    pub(crate) fn new(de: &'a mut Deserializer<R>) -> Self {
        Self { de, cached: None }
    }
}

impl<'a, R> AsMut<Deserializer<R>> for TransparentVecAccess<'a, R> {
    fn as_mut(&mut self) -> &mut Deserializer<R> {
        self.de
    }
}

impl<'a, 'de, R: Read<'de>> de::SeqAccess<'de> for TransparentVecAccess<'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.de.reader.peek().map(|b| b.try_into()).transpose()? {
            Some(EncodingCodes::DescribedType) => {
                let peek = PeekDescriptor::deserialize(self.as_mut())?;
                let peek = PeekTypeCode::Composite(peek);
                match &self.cached {
                    Some(cached) => {
                        if *cached != peek {
                            return Ok(None); // Treat as end of vec
                        }
                    }
                    None => self.cached = Some(peek),
                }
            }
            Some(code) => {
                let peek = PeekTypeCode::Primitive(code.into());
                match &self.cached {
                    Some(cached) => {
                        if *cached != peek {
                            return Ok(None); // Treat as end of vec
                        }
                    }
                    None => self.cached = Some(peek),
                }
            }
            None => return Ok(None),
        }

        seed.deserialize(self.as_mut()).map(Some)
    }
}

/// Accessor for map type
#[derive(Debug)]
pub struct MapAccess<'a, R> {
    de: &'a mut Deserializer<R>,
    _size: usize,
    count: usize,
}

impl<'a, R> MapAccess<'a, R> {
    pub(crate) fn new(de: &'a mut Deserializer<R>, size: usize, count: usize) -> Self {
        Self {
            de,
            _size: size,
            count,
        }
    }
}

impl<'a, R> AsMut<Deserializer<R>> for MapAccess<'a, R> {
    fn as_mut(&mut self) -> &mut Deserializer<R> {
        self.de
    }
}

impl<'a, 'de, R: Read<'de>> de::MapAccess<'de> for MapAccess<'a, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.count {
            0 => Ok(None),
            _ => {
                self.count -= 1;
                seed.deserialize(self.as_mut()).map(Some)
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.count -= 1;
        seed.deserialize(self.as_mut())
    }

    fn next_entry_seed<K, V>(
        &mut self,
        kseed: K,
        vseed: V,
    ) -> Result<Option<(K::Value, V::Value)>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
        V: de::DeserializeSeed<'de>,
    {
        match self.count {
            0 => Ok(None),
            _ => {
                // AMQP map count includes both key and value
                self.count -= 2;
                let key = kseed.deserialize(self.as_mut())?;
                let val = vseed.deserialize(self.as_mut())?;
                Ok(Some((key, val)))
            }
        }
    }
}

/// Accessor for enum variant
#[derive(Debug)]
pub struct VariantAccess<'a, R> {
    de: &'a mut Deserializer<R>,
}

impl<'a, R> VariantAccess<'a, R> {
    pub(crate) fn new(de: &'a mut Deserializer<R>) -> Self {
        Self { de }
    }
}

impl<'a, R> AsMut<Deserializer<R>> for VariantAccess<'a, R> {
    fn as_mut(&mut self) -> &mut Deserializer<R> {
        self.de
    }
}

impl<'a, 'de, R: Read<'de>> de::EnumAccess<'de> for VariantAccess<'a, R> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(mut self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let val = seed.deserialize(self.as_mut())?;
        Ok((val, self))
    }
}

impl<'a, 'de, R: Read<'de>> de::VariantAccess<'de> for VariantAccess<'a, R> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self.de, len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }
}

/// A special visitor access to the `Described` type
#[derive(Debug)]
pub struct DescribedAccess<'a, R> {
    de: &'a mut Deserializer<R>,
    counter: u32,
    field_count: u32,
}

impl<'a, 'de, R: Read<'de>> DescribedAccess<'a, R> {
    /// There will be at least one descriptor, and the length of the
    /// remaining items will be determined from the bytes
    pub(crate) fn list(de: &'a mut Deserializer<R>) -> Self {
        Self {
            de,
            field_count: 1,
            counter: 0,
        }
    }

    pub(crate) fn basic(de: &'a mut Deserializer<R>, field_count: u32) -> Self {
        Self {
            de,
            field_count,
            counter: 0,
        }
    }

    pub(crate) fn map(de: &'a mut Deserializer<R>) -> Self {
        Self {
            de,
            field_count: 1,
            counter: 0,
        }
    }

    pub(crate) fn consume_list_header(&mut self) -> Result<u32, Error> {
        // consume the list headers if
        match self
            .as_mut()
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??
        {
            EncodingCodes::List0 => Ok(0),
            EncodingCodes::List8 => {
                let _size = self
                    .as_mut()
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let count = self
                    .as_mut()
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                Ok(count as u32)
            }
            EncodingCodes::List32 => {
                let bytes = self
                    .as_mut()
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let _size = u32::from_be_bytes(bytes);
                let bytes = self
                    .as_mut()
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let count = u32::from_be_bytes(bytes);
                Ok(count)
            }
            _ => Err(de::Error::custom("Invalid format code. Expecting a list")),
        }
    }

    pub(crate) fn consume_map_header(&mut self) -> Result<u32, Error> {
        // consume the list headers if
        match self
            .as_mut()
            .get_elem_code_or_read_format_code()
            .ok_or_else(|| Error::unexpected_eof("Expecting format code"))??
        {
            EncodingCodes::Map8 => {
                let _size = self
                    .as_mut()
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let count = self
                    .as_mut()
                    .reader
                    .next()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                Ok(count as u32)
            }
            EncodingCodes::Map32 => {
                let bytes = self
                    .as_mut()
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let _size = u32::from_be_bytes(bytes);
                let bytes = self
                    .as_mut()
                    .reader
                    .read_const_bytes()
                    .ok_or_else(|| Error::unexpected_eof(""))?;
                let count = u32::from_be_bytes(bytes);
                Ok(count)
            }
            _ => Err(de::Error::custom("Invalid format code. Expecting a list")),
        }
    }
}

impl<'a, R> AsMut<Deserializer<R>> for DescribedAccess<'a, R> {
    fn as_mut(&mut self) -> &mut Deserializer<R> {
        self.de
    }
}

impl<'a, 'de, R: Read<'de>> de::SeqAccess<'de> for DescribedAccess<'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.counter >= self.field_count {
            return Ok(None);
        }
        let byte = match self.de.reader.peek() {
            Some(b) => b,
            None => return Ok(None),
        };
        let code = byte.try_into()?;
        let result = match code {
            EncodingCodes::DescribedType => {
                let result = seed.deserialize(self.as_mut()).map(Some);
                // The list header should only be consume once for each list
                // The sublist will create new DescribedAccess and thus take care of their own
                // list headers
                if self.counter == 0 {
                    if let StructEncoding::DescribedList = self.de.struct_encoding {
                        self.field_count += self.consume_list_header()?;
                    }
                }
                result
            }
            _ => seed.deserialize(self.as_mut()).map(Some),
        };

        self.counter += 1;

        result
    }
}

impl<'a, 'de, R: Read<'de>> de::MapAccess<'de> for DescribedAccess<'a, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        // Descriptor will only be deserialized as a key
        if self.counter >= self.field_count {
            return Ok(None);
        }
        let byte = match self.de.reader.peek() {
            Some(b) => b,
            None => return Ok(None),
        };
        let code = byte.try_into()?;
        let result = match code {
            EncodingCodes::Null => {
                let _ = self.de.reader.next(); // consume the Null byte
                Ok(None)
            }
            EncodingCodes::DescribedType => {
                self.de.enum_type = EnumType::Descriptor;
                let result = seed.deserialize(self.as_mut()).map(Some);
                if self.counter == 0 {
                    if let StructEncoding::DescribedMap = self.de.struct_encoding {
                        self.field_count += self.consume_map_header()?;
                    }
                }
                result
            }
            _ => seed.deserialize(self.as_mut()).map(Some),
        };

        self.counter += 1;

        result
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        if self.counter >= self.field_count {
            return Err(de::Error::custom("Invalid length. Expecting value"));
        }
        self.counter += 1;
        seed.deserialize(self.as_mut())
    }

    fn next_entry_seed<K, V>(
        &mut self,
        kseed: K,
        vseed: V,
    ) -> Result<Option<(K::Value, V::Value)>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
        V: de::DeserializeSeed<'de>,
    {
        if self.counter >= self.field_count {
            return Ok(None);
        }
        let byte = match self.de.reader.peek() {
            Some(b) => b,
            None => return Ok(None),
        };
        let code = byte.try_into()?;

        match code {
            EncodingCodes::Null => {
                let _ = self.de.reader.next(); // consume the Null byte
                Ok(None)
            }
            _ => {
                let key = kseed.deserialize(self.as_mut())?;
                let value = vseed.deserialize(self.as_mut())?;
                Ok(Some((key, value)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{de::DeserializeOwned, Deserialize};

    use crate::format_code::EncodingCodes;

    use super::{from_reader, from_slice};

    fn assert_eq_from_reader_vs_expected<'de, T>(buf: &'de [u8], expected: T)
    where
        T: DeserializeOwned + std::fmt::Debug + PartialEq,
    {
        let deserialized: T = from_reader(buf).unwrap();
        assert_eq!(deserialized, expected);
    }

    fn assert_eq_from_slice_vs_expected<'de, T>(buf: &'de [u8], expected: T)
    where
        T: Deserialize<'de> + std::fmt::Debug + PartialEq,
    {
        let deserialized: T = from_slice(buf).unwrap();
        assert_eq!(deserialized, expected)
    }

    #[test]
    fn test_deserialize_bool() {
        let buf = &[EncodingCodes::BooleanFalse as u8];
        let expected = false;
        assert_eq_from_reader_vs_expected(buf, expected);

        let buf = &[EncodingCodes::BooleanTrue as u8];
        let expected = true;
        assert_eq_from_reader_vs_expected(buf, expected);

        let buf = &[EncodingCodes::Boolean as u8, 1];
        let expected = true;
        assert_eq_from_reader_vs_expected(buf, expected);

        let buf = &[EncodingCodes::Boolean as u8, 0];
        let expected = false;
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_i8() {
        let buf = &[EncodingCodes::Byte as u8, 7i8 as u8];
        let expected = 7i8;
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_i16() {
        let mut buf = vec![EncodingCodes::Short as u8];
        buf.append(&mut 307i16.to_be_bytes().to_vec());
        let expected = 307i16;
        assert_eq_from_reader_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_i32() {
        let expected = -1i32;
        let buf = crate::to_vec(&expected).unwrap();
        let deserialized: i32 = from_slice(&buf).unwrap();
        assert_eq!(expected, deserialized);

        let buf = &[EncodingCodes::SmallInt as u8, 7i32 as u8];
        let expected = 7i32;
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_i64() {
        let expected = -1i64;
        let buf = crate::to_vec(&expected).unwrap();
        let deserialized: i64 = from_slice(&buf).unwrap();
        assert_eq!(expected, deserialized);

        let buf = &[EncodingCodes::SmallLong as u8, 7i64 as u8];
        let expected = 7i64;
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_u8() {
        let buf = &[EncodingCodes::UByte as u8, 5u8];
        let expected = 5u8;
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_u16() {
        let mut buf = vec![EncodingCodes::UShort as u8];
        buf.append(&mut 300u16.to_be_bytes().to_vec());
        let expected = 300u16;
        assert_eq_from_reader_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_u32() {
        let buf = &[EncodingCodes::UInt0 as u8];
        let expected = 0u32;
        assert_eq_from_reader_vs_expected(buf, expected);

        let buf = &[EncodingCodes::SmallUInt as u8, 5u8];
        let expected = 5u32;
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_u64() {
        let buf = &[EncodingCodes::ULong0 as u8];
        let expected = 0u64;
        assert_eq_from_reader_vs_expected(buf, expected);

        let buf = &[EncodingCodes::SmallULong as u8, 5u8];
        let expected = 5u64;
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    const SMALL_STRING_VALUE: &str = "Small String";
    const LARGE_STRING_VALUE: &str = r#"Large String: 
        "The quick brown fox jumps over the lazy dog. 
        "The quick brown fox jumps over the lazy dog. 
        "The quick brown fox jumps over the lazy dog. 
        "The quick brown fox jumps over the lazy dog. 
        "The quick brown fox jumps over the lazy dog. 
        "The quick brown fox jumps over the lazy dog. 
        "The quick brown fox jumps over the lazy dog. 
        "The quick brown fox jumps over the lazy dog."#;

    #[test]
    fn test_deserialize_str() {
        // str8
        let buf = [
            161u8, 12, 83, 109, 97, 108, 108, 32, 83, 116, 114, 105, 110, 103,
        ];
        let expected = SMALL_STRING_VALUE;
        assert_eq_from_slice_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_string() {
        // str8
        let buf = &[
            161u8, 12, 83, 109, 97, 108, 108, 32, 83, 116, 114, 105, 110, 103,
        ];
        let expected = SMALL_STRING_VALUE.to_string();
        assert_eq_from_reader_vs_expected(buf, expected);

        // str32
        let expected = LARGE_STRING_VALUE.to_string();
        let buf = crate::ser::to_vec(&expected).unwrap();
        assert_eq_from_reader_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_bytes() {
        use serde_bytes::ByteBuf;

        let buf = [EncodingCodes::VBin8 as u8, 4, 1, 2, 3, 4];
        let expected = ByteBuf::from(vec![1u8, 2, 3, 4]);
        assert_eq_from_slice_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_decimal32() {
        use crate::primitives::Dec32;
        use crate::ser::to_vec;

        let expected = Dec32::from([1, 2, 3, 4]);
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_slice_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_decimal() {
        use crate::primitives::{Dec128, Dec32, Dec64};
        use crate::ser::to_vec;

        let expected = Dec32::from([1, 2, 3, 4]);
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_slice_vs_expected(&buf, expected);

        let expected = Dec64::from([1, 2, 3, 4, 5, 6, 7, 8]);
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_slice_vs_expected(&buf, expected);

        let expected = Dec128::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_slice_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_uuid() {
        use crate::primitives::Uuid;
        use crate::ser::to_vec;

        let expected = Uuid::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_slice_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_timestamp() {
        use crate::primitives::Timestamp;
        use crate::ser::to_vec;

        let expected = Timestamp::from(0);
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_reader_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_symbol() {
        use crate::primitives::Symbol;
        let buf = &[0xa3 as u8, 0x04, 0x61, 0x6d, 0x71, 0x70];
        let expected = Symbol::from("amqp");
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_array() {
        use crate::primitives::Array;
        use crate::ser::to_vec;

        let expected = Array::from(vec![1i32, 2, 3, 4]);
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_reader_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_list() {
        // List0
        let expected: Vec<i32> = vec![];
        let buf = vec![EncodingCodes::List0 as u8];
        assert_eq_from_reader_vs_expected(&buf, expected);

        // List0
        let expected: &[i32; 0] = &[]; // slice will be (de)serialized as List
        let buf = vec![EncodingCodes::List0 as u8];
        assert_eq_from_reader_vs_expected(&buf, *expected);

        // List8
        let expected = vec![1i32, 2, 3, 4];
        let buf = vec![
            EncodingCodes::List8 as u8,
            1 + 4 * 2,
            4,
            EncodingCodes::SmallInt as u8,
            1,
            EncodingCodes::SmallInt as u8,
            2,
            EncodingCodes::SmallInt as u8,
            3,
            EncodingCodes::SmallInt as u8,
            4,
        ];
        assert_eq_from_reader_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_map() {
        use std::collections::BTreeMap;

        // Map should be considered ordered (here by its key)
        let buf = vec![
            EncodingCodes::Map8 as u8,
            1 + 4 * (3 + 2), // 1 for count, 4 kv pairs, 3 for "a", 2 for 1i32
            2 * 4,           // 4 kv pairs
            EncodingCodes::Str8 as u8,
            1,
            b'a', // fisrt key
            EncodingCodes::SmallInt as u8,
            1, // first value
            EncodingCodes::Str8 as u8,
            1,
            b'm',
            EncodingCodes::SmallInt as u8,
            2,
            EncodingCodes::Str8 as u8,
            1,
            b'p',
            EncodingCodes::SmallInt as u8,
            4,
            EncodingCodes::Str8 as u8,
            1,
            b'q',
            EncodingCodes::SmallInt as u8,
            3,
        ];
        let mut expected = BTreeMap::new();
        expected.insert("a".to_string(), 1i32);
        expected.insert("m".to_string(), 2);
        expected.insert("q".to_string(), 3);
        expected.insert("p".to_string(), 4);

        assert_eq_from_reader_vs_expected(&buf, expected);
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_deserialize_unit_struct_with_described_macro() {
        use crate as serde_amqp;
        use crate::macros::{DeserializeComposite, SerializeComposite};
        use crate::ser::to_vec;

        {
            #[derive(Debug, PartialEq, SerializeComposite, DeserializeComposite)]
            #[amqp_contract(code = "00:13", encoding = "list")]
            struct Foo;

            let foo = Foo;
            let buf = to_vec(&foo).unwrap();
            let foo2: Foo = from_slice(&buf).unwrap();
            assert_eq!(foo, foo2);
        }

        {
            #[derive(Debug, PartialEq, SerializeComposite, DeserializeComposite)]
            #[amqp_contract(code = "00:13", encoding = "list")]
            struct Foo();

            let foo = Foo();
            let buf = to_vec(&foo).unwrap();
            let foo2: Foo = from_slice(&buf).unwrap();
            assert_eq!(foo, foo2);
        }

        {
            #[derive(Debug, PartialEq, SerializeComposite, DeserializeComposite)]
            #[amqp_contract(code = "00:13", encoding = "list")]
            struct Foo {}

            let foo = Foo {};
            let buf = to_vec(&foo).unwrap();
            let foo2: Foo = from_slice(&buf).unwrap();
            assert_eq!(foo, foo2);
        }
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_deserialize_tuple_struct_with_described_macro() {
        use crate as serde_amqp;
        use crate::macros::{DeserializeComposite, SerializeComposite};
        use crate::ser::to_vec;

        #[derive(Debug, PartialEq, SerializeComposite, DeserializeComposite)]
        #[amqp_contract(code = "00:13", encoding = "list")]
        struct Foo(bool, i32);

        let foo = Foo(true, 9);
        let buf = to_vec(&foo).unwrap();
        let foo2: Foo = from_slice(&buf).unwrap();
        assert_eq!(foo, foo2);
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_deserialize_struct_with_described_macro() {
        use crate as serde_amqp;
        use crate::macros::{DeserializeComposite, SerializeComposite};
        use crate::ser::to_vec;

        #[derive(Debug, PartialEq, SerializeComposite, DeserializeComposite)]
        #[amqp_contract(code = "00:13", encoding = "list", rename_all = "kebab-case")]
        struct Foo {
            is_fool: bool,
            a: i32,
        }

        #[derive(Debug, PartialEq, SerializeComposite, DeserializeComposite)]
        #[amqp_contract(code = "00:09", encoding = "list", rename_all = "kebab-case")]
        struct Bar {
            is_fool: bool,
            a: i32,
        }

        let foo = Foo {
            is_fool: true,
            a: 9,
        };
        let buf = to_vec(&foo).unwrap();
        let foo2: Foo = from_slice(&buf).unwrap();
        assert_eq!(foo, foo2);

        let bar = Bar {
            is_fool: false,
            a: 13,
        };
        let buf = to_vec(&bar).unwrap();
        let bar2: Bar = from_slice(&buf).unwrap();
        assert_eq!(bar, bar2)
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_deserialize_composite_with_optional_fields() {
        use crate as serde_amqp;
        use crate::macros::DeserializeComposite;

        #[derive(Debug, DeserializeComposite)]
        #[amqp_contract(code = "0x00:0x13", encoding = "list")]
        struct Foo {
            pub is_fool: Option<bool>,
            pub a: Option<i32>,
        }

        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallULong as u8,
            0x13,
            EncodingCodes::List0 as u8,
        ];
        let foo: Foo = from_slice(&buf).unwrap();
        assert!(foo.is_fool.is_none());
        assert!(foo.a.is_none());

        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallULong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            2,
            1,
            EncodingCodes::BooleanTrue as u8,
        ];
        let foo: Foo = from_slice(&buf).unwrap();
        assert!(foo.is_fool.is_some());
        assert!(foo.a.is_none());

        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallULong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            4,
            2,
            EncodingCodes::BooleanTrue as u8,
            EncodingCodes::SmallInt as u8,
            1,
        ];
        let foo: Foo = from_slice(&buf).unwrap();
        assert!(foo.is_fool.is_some());
        assert!(foo.a.is_some());

        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallULong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            4,
            2,
            EncodingCodes::Null as u8,
            EncodingCodes::SmallInt as u8,
            1,
        ];
        let foo: Foo = from_slice(&buf).unwrap();
        assert!(foo.is_fool.is_none());
        assert!(foo.a.is_some());
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_deserialize_composite_tuple_with_optional_fields() {
        use crate as serde_amqp;
        use crate::macros::DeserializeComposite;

        #[derive(Debug, DeserializeComposite)]
        #[amqp_contract(code = "0x00:0x13", encoding = "list")]
        struct Foo(Option<bool>, Option<i32>);

        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallULong as u8,
            0x13,
            EncodingCodes::List0 as u8,
        ];
        let foo: Foo = from_slice(&buf).unwrap();
        assert!(foo.0.is_none());
        assert!(foo.1.is_none());

        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallULong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            2,
            1,
            EncodingCodes::BooleanTrue as u8,
        ];
        let foo: Foo = from_slice(&buf).unwrap();
        assert!(foo.0.is_some());
        assert!(foo.1.is_none());

        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallULong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            4,
            2,
            EncodingCodes::BooleanTrue as u8,
            EncodingCodes::SmallInt as u8,
            1,
        ];
        let foo: Foo = from_slice(&buf).unwrap();
        assert!(foo.0.is_some());
        assert!(foo.1.is_some());

        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallULong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            4,
            2,
            EncodingCodes::Null as u8,
            EncodingCodes::SmallInt as u8,
            1,
        ];
        let foo: Foo = from_slice(&buf).unwrap();
        assert!(foo.0.is_none());
        assert!(foo.1.is_some());

        #[derive(Debug, PartialEq, DeserializeComposite)]
        #[amqp_contract(code = "0x00:0x13", encoding = "list")]
        struct Bar {
            is_fool: Option<bool>,
            mandatory: u32,
            a: Option<i32>,
        }
        let bar = Bar {
            is_fool: None,
            mandatory: 0x13,
            a: None,
        };
        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallULong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            4,
            2,
            EncodingCodes::Null as u8,
            EncodingCodes::SmallUInt as u8,
            0x13,
        ];
        let bar2: Bar = from_slice(&buf).unwrap();
        assert_eq!(bar, bar2);
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_deserialize_basic_wrapper() {
        use crate as serde_amqp;
        use crate::macros::{DeserializeComposite, SerializeComposite};
        use crate::primitives::Symbol;
        use crate::ser::to_vec;
        use std::collections::BTreeMap;

        #[derive(Debug, SerializeComposite, DeserializeComposite, PartialEq)]
        #[amqp_contract(code = "0x00:0x01", encoding = "basic")]
        struct Wrapper(BTreeMap<Symbol, i32>);

        #[derive(Debug, SerializeComposite, DeserializeComposite, PartialEq)]
        #[amqp_contract(code = "0x00:0x1", encoding = "basic")]
        struct Wrapper2 {
            map: BTreeMap<Symbol, i32>,
        }

        let mut map = BTreeMap::new();
        map.insert(Symbol::from("a"), 1);
        map.insert(Symbol::from("b"), 2);
        let wrapper = Wrapper(map.clone());
        let buf = to_vec(&wrapper).unwrap();
        let wrapper1: Wrapper = from_slice(&buf).unwrap();
        assert_eq!(wrapper, wrapper1);

        let wrapper2 = Wrapper2 { map };
        let buf = to_vec(&wrapper2).unwrap();
        let wrapper3: Wrapper2 = from_slice(&buf).unwrap();
        assert_eq!(wrapper2, wrapper3);
    }

    #[test]
    fn test_deserialize_nondescribed_struct() {
        use crate::ser::to_vec;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Foo {
            bar: u32,
            is_fool: bool,
        }

        let expected = Foo {
            bar: 13,
            is_fool: true,
        };
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_reader_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_unit_variant() {
        use serde::{Deserialize, Serialize};

        use crate::ser::to_vec;

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A,
            B,
            C,
        }

        let foo = Foo::B;
        let buf = to_vec(&foo).unwrap();
        assert_eq_from_slice_vs_expected(&buf, foo);
    }

    #[test]
    fn test_deserialize_newtype_variant() {
        use serde::{Deserialize, Serialize};

        use crate::ser::to_vec;

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A(String),
            B(u64),
        }

        let foo = Foo::B(13);
        let buf = to_vec(&foo).unwrap();
        assert_eq_from_slice_vs_expected(&buf, foo);
    }

    #[test]
    fn test_deserialize_tuple_variant() {
        use serde::{Deserialize, Serialize};

        use crate::ser::to_vec;

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A(u32, bool),
            B(i32, String),
        }
        let expected = Foo::B(13, "amqp".to_string());
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_reader_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_struct_variant() {
        use serde::{Deserialize, Serialize};

        use crate::ser::to_vec;

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A { num: u32, is_a: bool },
            B { signed_num: i32, amqp: String },
        }
        let expected = Foo::A {
            num: 13,
            is_a: true,
        };
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_reader_vs_expected(&buf, expected);
    }
}
