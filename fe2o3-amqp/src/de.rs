use serde::de;
use std::convert::TryInto;

use crate::{
    error::Error,
    format::{ArrayWidth, Category, CompoundWidth, FixedWidth, VariableWidth},
    format_code::EncodingCodes,
    read::{IoReader, Read},
};

pub fn from_slice<'de, T: de::Deserialize<'de>>(slice: &'de [u8]) -> Result<T, Error> {
    let io_reader = IoReader::new(slice);
    let mut de = Deserializer::new(io_reader);
    T::deserialize(&mut de)
}

pub struct Deserializer<R> {
    reader: R,
}

impl<'de, R: Read<'de>> Deserializer<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    fn read_format_code(&mut self) -> Result<EncodingCodes, Error> {
        let code = self.reader.next();
        let code = code?;
        code.try_into()
    }

    #[inline]
    fn parse_bool(&mut self) -> Result<bool, Error> {
        // TODO: check whether is parsing in an array
        match self.read_format_code()? {
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
        match self.read_format_code()? {
            EncodingCodes::Byte => {
                let byte = self.reader.next()?;
                Ok(byte as i8)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_i16(&mut self) -> Result<i16, Error> {
        match self.read_format_code()? {
            EncodingCodes::Short => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(i16::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_i32(&mut self) -> Result<i32, Error> {
        match self.read_format_code()? {
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
        match self.read_format_code()? {
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
        match self.read_format_code()? {
            EncodingCodes::Ubyte => {
                let byte = self.reader.next()?;
                Ok(byte)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_u16(&mut self) -> Result<u16, Error> {
        match self.read_format_code()? {
            EncodingCodes::Ushort => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(u16::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_u32(&mut self) -> Result<u32, Error> {
        match self.read_format_code()? {
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
        match self.read_format_code()? {
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
        match self.read_format_code()? {
            EncodingCodes::Float => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(f32::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_f64(&mut self) -> Result<f64, Error> {
        match self.read_format_code()? {
            EncodingCodes::Double => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(f64::from_be_bytes(bytes))
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    #[inline]
    fn parse_char(&mut self) -> Result<char, Error> {
        match self.read_format_code()? {
            EncodingCodes::Char => {
                let bytes = self.reader.read_const_bytes()?;
                let n = u32::from_be_bytes(bytes);
                char::from_u32(n).ok_or(Error::InvalidValue)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    fn parse_string(&mut self) -> Result<String, Error> {
        match self.read_format_code()? {
            EncodingCodes::Str8 => {
                // read length byte
                let len = self.reader.next()?;
                let buf = self.reader.read_bytes(len as usize)?;
                String::from_utf8(buf).map_err(Into::into)
            }
            EncodingCodes::Str32 => {
                let len_bytes = self.reader.read_const_bytes()?;
                let len = u32::from_be_bytes(len_bytes);
                let buf = self.reader.read_bytes(len as usize)?;
                String::from_utf8(buf).map_err(Into::into)
            }
            _ => Err(Error::InvalidFormatCode),
        }
    }

    fn parse_byte_buf(&mut self) -> Result<Vec<u8>, Error> {
        match self.read_format_code()? {
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
            },
            EncodingCodes::Ulong | EncodingCodes::SmallUlong | EncodingCodes::Ulong0 => {
                self.deserialize_u64(visitor)
            },
            EncodingCodes::Float => self.deserialize_f32(visitor),
            EncodingCodes::Double => self.deserialize_f64(visitor),
            EncodingCodes::Char => self.deserialize_char(visitor),
            EncodingCodes::Str32 | EncodingCodes::Str8 => self.deserialize_string(visitor),
            EncodingCodes::VBin32 | EncodingCodes::VBin8 => self.deserialize_byte_buf(visitor),
            EncodingCodes::Null => self.deserialize_unit(visitor),

            // unimplemented
            EncodingCodes::Sym32 | EncodingCodes::Sym8 => todo!(),
            EncodingCodes::DescribedType => todo!(),
            EncodingCodes::Decimal32 => todo!(),
            EncodingCodes::Decimal64 => todo!(),
            EncodingCodes::Decimal128 => todo!(),
            EncodingCodes::Timestamp => todo!(),
            EncodingCodes::Uuid => todo!(),
            EncodingCodes::Array32 | EncodingCodes::Array8 => todo!(),
            EncodingCodes::List0 | EncodingCodes::List8 | EncodingCodes::List32 => todo!(),
            EncodingCodes::Map32 | EncodingCodes::Map8 => todo!()
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
        visitor.visit_i64(self.parse_i64()?)
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
        visitor.visit_string(self.parse_string()?)
    }

    #[inline]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // TODO: considering adding a buffer to the reader
        visitor.visit_str(&self.parse_string()?)
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
        // TODO: considering adding a buffer to the reader
        visitor.visit_bytes(&self.parse_byte_buf()?)
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
        match self.read_format_code()? {
            EncodingCodes::Null => visitor.visit_unit(),
            _ => Err(Error::InvalidFormatCode),
        }
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
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
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
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
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
        todo!()
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    // an identifier is either a field of a struct or a variant of an eunm
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::{format_code::EncodingCodes, ser::to_vec};

    use super::from_slice;

    fn assert_eq_deserialized_vs_expected<'de, T>(buf: &'de [u8], expected: T)
    where
        T: Deserialize<'de> + std::fmt::Debug + PartialEq,
    {
        let deserialized: T = from_slice(buf).unwrap();
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_deserialize_bool() {
        let buf = &[EncodingCodes::BooleanFalse as u8];
        let expected = false;
        assert_eq_deserialized_vs_expected(buf, expected);

        let buf = &[EncodingCodes::BooleanTrue as u8];
        let expected = true;
        assert_eq_deserialized_vs_expected(buf, expected);

        let buf = &[EncodingCodes::Boolean as u8, 1];
        let expected = true;
        assert_eq_deserialized_vs_expected(buf, expected);

        let buf = &[EncodingCodes::Boolean as u8, 0];
        let expected = false;
        assert_eq_deserialized_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_i8() {
        let buf = &[EncodingCodes::Byte as u8, 7i8 as u8];
        let expected = 7i8;
        assert_eq_deserialized_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_i16() {
        let mut buf = vec![EncodingCodes::Short as u8];
        buf.append(&mut 307i16.to_be_bytes().to_vec());
        let expected = 307i16;
        assert_eq_deserialized_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_i32() {
        let buf = &[EncodingCodes::SmallInt as u8, 7i32 as u8];
        let expected = 7i32;
        assert_eq_deserialized_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_i64() {
        let buf = &[EncodingCodes::SmallLong as u8, 7i64 as u8];
        let expected = 7i64;
        assert_eq_deserialized_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_u8() {
        let buf = &[EncodingCodes::Ubyte as u8, 5u8];
        let expected = 5u8;
        assert_eq_deserialized_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_u16() {
        let mut buf = vec![EncodingCodes::Ushort as u8];
        buf.append(&mut 300u16.to_be_bytes().to_vec());
        let expected = 300u16;
        assert_eq_deserialized_vs_expected(&buf, expected);
    }

    #[test]
    fn test_deserialize_u32() {
        let buf = &[EncodingCodes::Uint0 as u8];
        let expected = 0u32;
        assert_eq_deserialized_vs_expected(buf, expected);

        let buf = &[EncodingCodes::SmallUint as u8, 5u8];
        let expected = 5u32;
        assert_eq_deserialized_vs_expected(buf, expected);
    }

    #[test]
    fn test_deserialize_u64() {
        let buf = &[EncodingCodes::Ulong0 as u8];
        let expected = 0u64;
        assert_eq_deserialized_vs_expected(buf, expected);

        let buf = &[EncodingCodes::SmallUlong as u8, 5u8];
        let expected = 5u64;
        assert_eq_deserialized_vs_expected(buf, expected);
    }
}
