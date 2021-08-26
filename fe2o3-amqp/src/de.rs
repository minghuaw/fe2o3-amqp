use serde::de::{self};
use std::convert::TryInto;

use crate::{
    error::Error,
    fixed_width::{DECIMAL128_WIDTH, DECIMAL32_WIDTH, DECIMAL64_WIDTH, UUID_WIDTH},
    format::{
        OFFSET_ARRAY32, OFFSET_ARRAY8, OFFSET_LIST32, OFFSET_LIST8, OFFSET_MAP32, OFFSET_MAP8,
    },
    format_code::EncodingCodes,
    read::{IoReader, Read, SliceReader},
    types::{ARRAY, DESCRIBED_FIELDS, DESCRIPTOR, DESERIALIZE_DESCRIBED},
    types::{DECIMAL128, DECIMAL32, DECIMAL64, ENCODING_TYPE, SYMBOL, TIMESTAMP, UUID},
    util::{EnumType, NewType},
    value::VALUE,
};

pub fn from_reader<T: de::DeserializeOwned>(reader: impl std::io::Read) -> Result<T, Error> {
    let reader = IoReader::new(reader);
    let mut de = Deserializer::new(reader);
    T::deserialize(&mut de)
}

pub fn from_slice<'de, T: de::Deserialize<'de>>(slice: &'de [u8]) -> Result<T, Error> {
    let reader = SliceReader::new(slice);
    let mut de = Deserializer::new(reader);
    T::deserialize(&mut de)
}

pub struct Deserializer<R> {
    reader: R,
    new_type: NewType,
    enum_type: EnumType,
    elem_format_code: Option<EncodingCodes>,
}

impl<'de, R: Read<'de>> Deserializer<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            new_type: Default::default(),
            enum_type: Default::default(),
            elem_format_code: None,
        }
    }

    pub fn symbol(reader: R) -> Self {
        Self {
            reader,
            new_type: NewType::Symbol,
            enum_type: Default::default(),
            elem_format_code: None,
        }
    }

    fn read_format_code(&mut self) -> Result<EncodingCodes, Error> {
        let code = self.reader.next();
        let code = code?;
        code.try_into()
    }

    fn get_elem_code_or_read_format_code(&mut self) -> Result<EncodingCodes, Error> {
        match &self.elem_format_code {
            Some(c) => Ok(c.clone()),
            None => self.read_format_code(),
        }
    }

    fn get_elem_code_or_peek_byte(&mut self) -> Result<u8, Error> {
        match &self.elem_format_code {
            Some(c) => Ok(c.clone() as u8),
            None => self.reader.peek(),
        }
    }

    #[inline]
    fn parse_bool(&mut self) -> Result<bool, Error> {
        // TODO: check whether is parsing in an array
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Boolean => {
                let byte = self.reader.next()?;
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
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Byte => {
                let byte = self.reader.next()?;
                Ok(byte as i8)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_i16(&mut self) -> Result<i16, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Short => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(i16::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_i32(&mut self) -> Result<i32, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Int => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(i32::from_be_bytes(bytes))
            }
            EncodingCodes::SmallInt => {
                let byte = self.reader.next()?;
                Ok(byte as i32)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_i64(&mut self) -> Result<i64, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Long => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(i64::from_be_bytes(bytes))
            }
            EncodingCodes::SmallLong => {
                let byte = self.reader.next()?;
                Ok(byte as i64)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_u8(&mut self) -> Result<u8, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Ubyte => {
                let byte = self.reader.next()?;
                Ok(byte)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_u16(&mut self) -> Result<u16, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Ushort => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(u16::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_u32(&mut self) -> Result<u32, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Uint => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(u32::from_be_bytes(bytes))
            }
            EncodingCodes::SmallUint => {
                let byte = self.reader.next()?;
                Ok(byte as u32)
            }
            EncodingCodes::Uint0 => Ok(0),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_u64(&mut self) -> Result<u64, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Ulong => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(u64::from_be_bytes(bytes))
            }
            EncodingCodes::SmallUlong => {
                let byte = self.reader.next()?;
                Ok(byte as u64)
            }
            EncodingCodes::Ulong0 => Ok(0),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_f32(&mut self) -> Result<f32, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Float => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(f32::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_f64(&mut self) -> Result<f64, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Double => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(f64::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_char(&mut self) -> Result<char, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Char => {
                let bytes = self.reader.read_const_bytes()?;
                let n = u32::from_be_bytes(bytes);
                char::from_u32(n).ok_or(Error::InvalidValue)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    fn read_small_string(&mut self) -> Result<String, Error> {
        let len = self.reader.next()?;
        let buf = self.reader.read_bytes(len as usize)?;
        String::from_utf8(buf).map_err(Into::into)
    }

    fn read_string(&mut self) -> Result<String, Error> {
        let len_bytes = self.reader.read_const_bytes()?;
        let len = u32::from_be_bytes(len_bytes);
        let buf = self.reader.read_bytes(len as usize)?;
        String::from_utf8(buf).map_err(Into::into)
    }

    fn parse_string(&mut self) -> Result<String, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Str8 => self.read_small_string(),
            EncodingCodes::Str32 => self.read_string(),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    fn parse_symbol(&mut self) -> Result<String, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Sym8 => self.read_small_string(),
            EncodingCodes::Sym32 => self.read_string(),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    fn parse_byte_buf(&mut self) -> Result<Vec<u8>, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::VBin8 => {
                let len = self.reader.next()?;
                self.reader.read_bytes(len as usize)
            }
            EncodingCodes::VBin32 => {
                let len_bytes = self.reader.read_const_bytes()?;
                let len = u32::from_be_bytes(len_bytes);
                self.reader.read_bytes(len as usize)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    fn parse_decimal(&mut self) -> Result<Vec<u8>, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Decimal32 => self.reader.read_bytes(DECIMAL32_WIDTH),
            EncodingCodes::Decimal64 => self.reader.read_bytes(DECIMAL64_WIDTH),
            EncodingCodes::Decimal128 => self.reader.read_bytes(DECIMAL128_WIDTH),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    fn parse_uuid(&mut self) -> Result<Vec<u8>, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Uuid => self.reader.read_bytes(UUID_WIDTH),
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_timestamp(&mut self) -> Result<i64, Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Timestamp => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(i64::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    fn parse_unit(&mut self) -> Result<(), Error> {
        match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Null => Ok(()),
            _ => Err(Error::InvalidFormatCode),
        }
    }
}

impl<'de, 'a, R> de::Deserializer<'de> for &'a mut Deserializer<R>
where
    R: Read<'de>,
{
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.reader.peek()?.try_into()? {
            EncodingCodes::Boolean | EncodingCodes::BooleanFalse | EncodingCodes::BooleanTrue => {
                self.deserialize_bool(visitor)
            }
            EncodingCodes::Byte => self.deserialize_i8(visitor),
            EncodingCodes::Short => self.deserialize_i16(visitor),
            EncodingCodes::Int | EncodingCodes::SmallInt => self.deserialize_i32(visitor),
            EncodingCodes::Long | EncodingCodes::SmallLong => self.deserialize_i64(visitor),
            EncodingCodes::Ubyte => self.deserialize_u8(visitor),
            EncodingCodes::Ushort => self.deserialize_u16(visitor),
            EncodingCodes::Uint | EncodingCodes::SmallUint | EncodingCodes::Uint0 => {
                self.deserialize_u32(visitor)
            }
            EncodingCodes::Ulong | EncodingCodes::SmallUlong | EncodingCodes::Ulong0 => {
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
                self.deserialize_struct(DESERIALIZE_DESCRIBED, DESCRIBED_FIELDS, visitor)
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
            NewType::None => visitor.visit_i64(self.parse_i64()?),
            NewType::Timestamp => {
                self.new_type = NewType::None;
                visitor.visit_i64(self.parse_timestamp()?)
            }
            _ => unreachable!(),
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
        println!(">>> Debug deserialize_u64");
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
        println!(">>> Debug: deserialize_string");

        match self.new_type {
            NewType::None => visitor.visit_string(self.parse_string()?),
            NewType::Symbol => {
                self.new_type = NewType::None;
                visitor.visit_string(self.parse_symbol()?)
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // // TODO: considering adding a buffer to the reader
        println!(">>> Debug: deserialize_str");
        let len = match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::Str8 => self.reader.next()? as usize,
            EncodingCodes::Str32 => {
                let len_bytes = self.reader.read_const_bytes()?;
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
        println!(">>> Debug deserialize_byte_buf");
        match self.new_type {
            NewType::None => visitor.visit_byte_buf(self.parse_byte_buf()?),
            NewType::Dec32 | NewType::Dec64 | NewType::Dec128 => {
                self.new_type = NewType::None;
                visitor.visit_byte_buf(self.parse_decimal()?)
            }
            NewType::Uuid => {
                self.new_type = NewType::None;
                visitor.visit_byte_buf(self.parse_uuid()?)
            }
            _ => unreachable!(),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        println!(">>> Debug deserialize_bytes");
        let len = match self.get_elem_code_or_read_format_code()? {
            EncodingCodes::VBin8 => self.reader.next()? as usize,
            EncodingCodes::VBin32 => {
                let bytes = self.reader.read_const_bytes()?;
                u32::from_be_bytes(bytes) as usize
            }
            _ => return Err(Error::InvalidFormatCode),
        };
        self.reader.forward_read_bytes(len, visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.reader.peek()?.try_into()? {
            EncodingCodes::Null => visitor.visit_none(),
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
        println!(">>> Debug deserialize_newtype_struct {:?}", name);
        if name == SYMBOL {
            self.new_type = NewType::Symbol;
        } else if name == DECIMAL32 {
            self.new_type = NewType::Dec32;
        } else if name == DECIMAL64 {
            self.new_type = NewType::Dec64;
        } else if name == DECIMAL128 {
            self.new_type = NewType::Dec128;
        } else if name == UUID {
            self.new_type = NewType::Uuid;
        } else if name == TIMESTAMP {
            self.new_type = NewType::Timestamp;
        }
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        println!(">>> Debug deserialize_seq");

        let code = self.get_elem_code_or_read_format_code()?;

        match code {
            EncodingCodes::Array8 => {
                println!(">>> Debug: Array8");
                // Read "header" bytes
                let len = self.reader.next()? as usize;
                let count = self.reader.next()? as usize;
                let format_code = self.read_format_code()?;
                self.elem_format_code = Some(format_code);

                // Account for offset
                let len = len - OFFSET_ARRAY8;
                // let buf = self.reader.read_bytes(len)?;

                visitor.visit_seq(ArrayAccess::new(self, len, count))
            }
            EncodingCodes::Array32 => {
                println!(">>> Debug: Array32");
                // Read "header" bytes
                let len_bytes = self.reader.read_const_bytes()?;
                let count_bytes = self.reader.read_const_bytes()?;
                let format_code = self.read_format_code()?;
                self.elem_format_code = Some(format_code);

                // Conversion
                let len = u32::from_be_bytes(len_bytes) as usize;
                let count = u32::from_be_bytes(count_bytes) as usize;

                // Account for offset
                let len = len as usize - OFFSET_ARRAY32;
                // let buf = self.reader.read_bytes(len)?;

                visitor.visit_seq(ArrayAccess::new(self, len, count))
            }
            EncodingCodes::List0 => {
                let len = 0;
                let count = 0;
                visitor.visit_seq(ListAccess::new(self, len, count))
            }
            EncodingCodes::List8 => {
                let len = self.reader.next()? as usize;
                let count = self.reader.next()? as usize;

                // Account for offset
                let len = len - OFFSET_LIST8;

                // Make sure there is no other element format code
                self.elem_format_code = None;
                visitor.visit_seq(ListAccess::new(self, len, count))
            }
            EncodingCodes::List32 => {
                let len_bytes = self.reader.read_const_bytes()?;
                let count_bytes = self.reader.read_const_bytes()?;
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
        println!(">>> Debug deserialize_tuple");

        // Tuple will always be deserialized as List
        let code = self.get_elem_code_or_read_format_code()?;

        let (size, count) = match code {
            EncodingCodes::List0 => {
                let size = 0;
                let count = 0;
                (size, count)
            }
            EncodingCodes::List8 => {
                let size = self.reader.next()? as usize;
                let count = self.reader.next()? as usize;

                // Account for offset
                let size = size - OFFSET_LIST8;

                // Make sure there is no other element format code
                self.elem_format_code = None;
                (size, count)
            }
            EncodingCodes::List32 => {
                let size_bytes = self.reader.read_const_bytes()?;
                let count_bytes = self.reader.read_const_bytes()?;
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

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let code = self.get_elem_code_or_read_format_code()?;
        println!("{:?}", &code);
        let (size, count) = match code {
            EncodingCodes::Map8 => {
                let size = self.reader.next()? as usize;
                let count = self.reader.next()? as usize;

                // Account for offset
                let size = size - OFFSET_MAP8;

                (size, count)
            }
            EncodingCodes::Map32 => {
                let size_bytes = self.reader.read_const_bytes()?;
                let count_bytes = self.reader.read_const_bytes()?;

                let size = u32::from_be_bytes(size_bytes) as usize;
                let count = u32::from_be_bytes(count_bytes) as usize;

                // Account for offset
                let size = size - OFFSET_MAP32;

                (size, count)
            }
            _ => return Err(Error::InvalidFormatCode),
        };

        // AMQP map count includes both key and value, should be halfed
        let count = count / 2;
        visitor.visit_map(MapAccess::new(self, size, count))
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
        match self.reader.peek()?.try_into()? {
            EncodingCodes::List0 | EncodingCodes::List32 | EncodingCodes::List8 => {
                self.deserialize_tuple(fields.len(), visitor)
            }
            EncodingCodes::Map32 | EncodingCodes::Map8 => self.deserialize_map(visitor),
            EncodingCodes::DescribedType => {
                if name != DESERIALIZE_DESCRIBED {
                    return Err(Error::InvalidFormatCode);
                }
                self.reader.next()?;
                visitor.visit_seq(DescribedAccess::new(self))
            }
            _ => Err(Error::InvalidFormatCode),
        }
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
        println!(">>> Debug deserialize_enum");
        if name == VALUE {
            self.enum_type = EnumType::Value;
            visitor.visit_enum(VariantAccess::new(self))
        } else if name == DESCRIPTOR {
            self.enum_type = EnumType::Descriptor;
            visitor.visit_enum(VariantAccess::new(self))
        } else if name == ENCODING_TYPE {
            visitor.visit_enum(VariantAccess::new(self))
        } else {
            // TODO: Considering the following enum serialization format
            // `unit_variant` - a single u32
            // generic `newtype_variant` - List([u32, Value])
            // `tuple_variant` and `struct_variant` - List([u32, List([Value, *])])
            match self.get_elem_code_or_peek_byte()?.try_into()? {
                EncodingCodes::Uint | EncodingCodes::Uint0 | EncodingCodes::SmallUint => {
                    visitor.visit_enum(VariantAccess::new(self))
                }
                EncodingCodes::List0 => Err(Error::InvalidFormatCode),
                EncodingCodes::List8 => {
                    let _code = self.reader.next()?;
                    let _size = self.reader.next()? as usize;
                    let count = self.reader.next()? as usize;
                    if count != 2 {
                        return Err(Error::InvalidLength);
                    }
                    visitor.visit_enum(VariantAccess::new(self))
                }
                EncodingCodes::List32 => {
                    let _code = self.reader.next()?;
                    let size_bytes = self.reader.read_const_bytes()?;
                    let _size = u32::from_be_bytes(size_bytes);
                    let count_bytes = self.reader.read_const_bytes()?;
                    let count = u32::from_be_bytes(count_bytes);

                    if count != 2 {
                        return Err(Error::InvalidLength);
                    }
                    visitor.visit_enum(VariantAccess::new(self))
                }
                _ => Err(Error::InvalidFormatCode),
            }
        }
    }

    // an identifier is either a field of a struct or a variant of an eunm
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.enum_type {
            EnumType::Value => {
                let code = self.get_elem_code_or_peek_byte()?;
                println!(">>> Debug deserialize_identifier {:x?}", &code);
                visitor.visit_u8(code)
            }
            EnumType::Descriptor => {
                let code = self.get_elem_code_or_peek_byte()?;
                visitor.visit_u8(code)
            }
            EnumType::None => {
                // The following are the possible identifiers
                match self.get_elem_code_or_peek_byte()?.try_into()? {
                    // If a struct is serialized as a map, then the fields are serialized as str
                    EncodingCodes::Str32 | EncodingCodes::Str8 => self.deserialize_str(visitor),
                    // FIXME: Enum variant currently are serialzied as map of with variant index and a list
                    EncodingCodes::Uint | EncodingCodes::SmallUint | EncodingCodes::Uint0 => {
                        self.deserialize_u32(visitor)
                    }
                    // Potentially using `Descriptor::Name` as identifier
                    EncodingCodes::Sym32 | EncodingCodes::Sym8 => {
                        self.new_type = NewType::Symbol;
                        self.deserialize_string(visitor)
                    }
                    // Potentially using `Descriptor::Code` as identifier
                    EncodingCodes::Ulong | EncodingCodes::SmallUlong | EncodingCodes::Ulong0 => {
                        self.deserialize_u64(visitor)
                    }
                    // Other types should not be used to serialize identifiers
                    _ => Err(Error::InvalidFormatCode),
                }
            }
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // The deserializer will only peek the next u8
        visitor.visit_u8(self.get_elem_code_or_peek_byte()?)
    }
}

pub struct ArrayAccess<'a, R> {
    de: &'a mut Deserializer<R>,
    _size: usize,
    count: usize,
    // buf: Vec<u8>,
}

impl<'a, R> ArrayAccess<'a, R> {
    pub fn new(
        de: &'a mut Deserializer<R>,
        size: usize,
        count: usize,
        // buf: Vec<u8>
    ) -> Self {
        Self {
            de,
            _size: size,
            count,
            // buf
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
        println!(">>> Debug: ArrayAccess::next_element_seed");
        match self.count {
            0 => {
                self.de.elem_format_code = None;
                Ok(None)
            }
            _ => {
                self.count = self.count - 1;
                seed.deserialize(self.as_mut()).map(Some)
            }
        }
    }
}

pub struct ListAccess<'a, R> {
    de: &'a mut Deserializer<R>,
    _size: usize,
    count: usize,
}

impl<'a, R> ListAccess<'a, R> {
    pub fn new(de: &'a mut Deserializer<R>, size: usize, count: usize) -> Self {
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
        println!(">>> Debug: ListAccess::next_element_seed");
        match self.count {
            0 => Ok(None),
            _ => {
                self.count = self.count - 1;
                seed.deserialize(self.as_mut()).map(Some)
            }
        }
    }
}

pub struct MapAccess<'a, R> {
    de: &'a mut Deserializer<R>,
    _size: usize,
    count: usize,
}

impl<'a, R> MapAccess<'a, R> {
    pub fn new(de: &'a mut Deserializer<R>, size: usize, count: usize) -> Self {
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

    fn next_key_seed<K>(&mut self, _seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        unreachable!()
    }

    fn next_value_seed<V>(&mut self, _seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        unreachable!()
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
                self.count = self.count - 1;
                let key = kseed.deserialize(self.as_mut())?;
                let val = vseed.deserialize(self.as_mut())?;
                Ok(Some((key, val)))
            }
        }
    }
}

pub struct VariantAccess<'a, R> {
    de: &'a mut Deserializer<R>,
}

impl<'a, R> VariantAccess<'a, R> {
    pub fn new(de: &'a mut Deserializer<R>) -> Self {
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
        println!(">>> Debug VariantAccess::unit_variant");
        // de::Deserialize::deserialize(self.de)
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        println!(">>> Debug VariantAccess::newtype_variant_seed");
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        println!(">>> Debug VariantAccess::tuple_variant");
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
        println!(">>> Debug VariantAccess::struct_variant");
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }
}

enum FieldRole {
    // The descriptor bytes should be consumed
    Descriptor,

    // The byte after the descriptor should be peeked to see
    // which type of encoding is used
    EncodingType,

    // The bytes after the descriptor should be consumed
    Value,

    // All fields should be already deserialized. Probably redundant
    End,
}

/// A special visitor access to the `Described` type
pub struct DescribedAccess<'a, R> {
    de: &'a mut Deserializer<R>,
    field_role: FieldRole,
}

impl<'a, R> DescribedAccess<'a, R> {
    pub fn new(de: &'a mut Deserializer<R>) -> Self {
        Self {
            de,
            // The first field should be the descriptor
            field_role: FieldRole::Descriptor,
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
        println!(">>> Debug: DescribedAccess::next_element_seed");
        match self.field_role {
            FieldRole::Descriptor => {
                self.field_role = FieldRole::EncodingType;
                seed.deserialize(self.as_mut()).map(Some)
            }
            FieldRole::EncodingType => {
                self.field_role = FieldRole::Value;
                seed.deserialize(self.as_mut()).map(Some)
            }
            FieldRole::Value => {
                self.field_role = FieldRole::End;
                seed.deserialize(self.as_mut()).map(Some)
            }
            FieldRole::End => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::panic;

    use serde::{de::DeserializeOwned, Deserialize};

    use crate::{
        format_code::EncodingCodes,
        types::Descriptor,
        types::{Described, EncodingType},
    };

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
        let buf = &[EncodingCodes::SmallInt as u8, 7i32 as u8];
        let expected = 7i32;
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_i64() {
        let buf = &[EncodingCodes::SmallLong as u8, 7i64 as u8];
        let expected = 7i64;
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_u8() {
        let buf = &[EncodingCodes::Ubyte as u8, 5u8];
        let expected = 5u8;
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_u16() {
        let mut buf = vec![EncodingCodes::Ushort as u8];
        buf.append(&mut 300u16.to_be_bytes().to_vec());
        let expected = 300u16;
        assert_eq_from_reader_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_u32() {
        let buf = &[EncodingCodes::Uint0 as u8];
        let expected = 0u32;
        assert_eq_from_reader_vs_expected(buf, expected);

        let buf = &[EncodingCodes::SmallUint as u8, 5u8];
        let expected = 5u32;
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_u64() {
        let buf = &[EncodingCodes::Ulong0 as u8];
        let expected = 0u64;
        assert_eq_from_reader_vs_expected(buf, expected);

        let buf = &[EncodingCodes::SmallUlong as u8, 5u8];
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
        use crate::ser::to_vec;
        use serde_bytes::{ByteBuf, Bytes};

        let val = [1u8, 2, 3, 4];
        let buf = to_vec(&Bytes::new(&val)).unwrap();
        println!("{:?}", buf);
        let recovered: ByteBuf = from_slice(&buf).unwrap();
        println!("{:?}", &recovered);
    }

    #[test]
    fn test_deserialize_decimal() {
        use crate::ser::to_vec;
        use crate::types::{Dec128, Dec32, Dec64};

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
        use crate::ser::to_vec;
        use crate::types::Uuid;

        let expected = Uuid::from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_slice_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_timestamp() {
        use crate::ser::to_vec;
        use crate::types::Timestamp;

        let expected = Timestamp::from(0);
        let buf = to_vec(&expected).unwrap();
        assert_eq_from_reader_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_symbol() {
        use crate::types::Symbol;
        let buf = &[0xa3 as u8, 0x04, 0x61, 0x6d, 0x71, 0x70];
        let expected = Symbol::from("amqp");
        assert_eq_from_reader_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_array() {
        use crate::ser::to_vec;
        use crate::types::Array;

        let expected = Array::from(vec![1i32, 2, 3, 4]);
        let buf = to_vec(&expected).unwrap();
        println!("{:x?}", &buf);
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

    #[test]
    fn test_deserialize_descriptor() {
        use crate::ser::to_vec;
        use crate::types::Symbol;

        let descriptor = Descriptor::Name(Symbol::from("amqp"));
        let buf = to_vec(&descriptor).unwrap();
        assert_eq_from_slice_vs_expected(&buf, descriptor);
    }

    #[test]
    fn test_deserialize_encoding_type() {
        let buf = [EncodingCodes::List8 as u8];
        let encoding_type: EncodingType = from_slice(&buf).unwrap();
        if let EncodingType::Basic | EncodingType::Map = encoding_type {
            panic!("Expecting EncodingType::List")
        }
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
    fn test_deserialize_described_list_struct() {
        use crate::ser::to_vec;
        use crate::types::EncodingType;
        use crate::types::Symbol;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Foo {
            a_field: i32,
            b: bool,
        }

        let foo = Foo {
            a_field: 13,
            b: false,
        };
        let descriptor = Descriptor::Name(Symbol::from("Foo"));
        let foo = Described::new(EncodingType::List, descriptor, foo);
        let buf = to_vec(&foo).unwrap();
        assert_eq_from_slice_vs_expected(&buf, foo);
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
        // let foo2: Foo = from_slice(&buf).unwrap();
        // println!("{:?}", &foo2);
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
        println!("{:x?}", &buf);
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
        // let foo: Foo = from_slice(&buf).unwrap();
        // println!("{:?}", foo);
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
        // let foo: Foo = from_slice(&buf).unwrap();
        // println!("{:?}", foo);
        assert_eq_from_reader_vs_expected(&buf, expected);
    }
}
