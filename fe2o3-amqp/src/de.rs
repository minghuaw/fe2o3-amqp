use std::{convert::TryInto};
use serde::de;

use crate::{format_code::EncodingCodes, error::Error, format::{ArrayWidth, CompoundWidth, FixedWidth, VariableWidth}, read::{IoReader, Read}};

pub fn from_slice<'de, T: de::Deserialize<'de>>(slice: &'de [u8]) -> Result<T, Error> {
    let io_reader = IoReader::new(slice);
    let mut de = Deserializer::new(io_reader);
    T::deserialize(&mut de)
}

pub struct CompoundBuf {
    count: u32,
    value_buf: Vec<u8>
}

pub struct ArrayBuf {
    count: u32,
    elem_code: u8, // element constructor
    value_buf: Vec<u8>,
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

    // fn read_fixed_width_bytes(&mut self, width: FixedWidth) -> Option<Result<Vec<u8>, Error>> {
    //     let n = width as usize;
    //     self.reader.read_bytes(n)
    // }

    fn read_variable_width_bytes(&mut self, width: VariableWidth) -> Result<Vec<u8>, Error> {
        let n = match width {
            VariableWidth::One => {
                let size = self.reader.next()?;
                size as usize
            },
            VariableWidth::Four => {
                let size_buf = self.reader.read_const_bytes::<4>()?;
                u32::from_be_bytes(size_buf) as usize
            }
        };

        self.reader.read_bytes(n)
    }

    fn read_compound_bytes(&mut self, width: CompoundWidth) -> Result<CompoundBuf, Error> {
        let cb = match width {
            CompoundWidth::Zero => {
                CompoundBuf { count: 0, value_buf: Vec::with_capacity(0) }
            },
            CompoundWidth::One => {
                let size = self.reader.next()?;
                let count = self.reader.next()?;

                // Need to subtract the one byte taken by `count`
                let size = size - 1;
                let value_buf = self.reader.read_bytes(size as usize)?;
                CompoundBuf {count: count as u32, value_buf }
            },
            CompoundWidth::Four => {
                let size_buf = self.reader.read_const_bytes::<4>()?;
                let count_buf = self.reader.read_const_bytes::<4>()?;
                let size = u32::from_be_bytes(size_buf);
                let count = u32::from_be_bytes(count_buf);

                // Need to substract the four byte taken by `count`
                let size = size - 4;
                let value_buf = self.reader.read_bytes(size as usize)?;
                CompoundBuf { count, value_buf }
            }
        };

        Ok(cb)
    }

    fn read_array_bytes(&mut self, width: ArrayWidth) -> Result<ArrayBuf, Error> {
        let ab = match width {
            ArrayWidth::One => {
                let size = self.reader.next()?;
                let count = self.reader.next()?;
                let elem_code = self.reader.next()?;

                // Must subtract the one byte taken by `count` and one byte taken by `elem_code`
                let size = size - 1 - 1;
                let value_buf = self.reader.read_bytes(size as usize)?;
                ArrayBuf { count: count as u32, elem_code, value_buf }
            },
            ArrayWidth::Four => {
                let size_buf = self.reader.read_const_bytes::<4>()?;
                let count_buf = self.reader.read_const_bytes::<4>()?;
                let elem_code = self.reader.next()?;

                let size = u32::from_be_bytes(size_buf);
                let count = u32::from_be_bytes(count_buf);

                // Must subtract the four bytes taken by `count` and one byte taken by `elem_code`
                let size = size - 4 - 1;
                let value_buf = self.reader.read_bytes(size as usize)?;
                ArrayBuf { count, elem_code, value_buf }
            }
        };

        Ok(ab)
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
                    _ => Err(Error::InvalidValue)
                }
            },
            EncodingCodes::BooleanTrue => Ok(true),
            EncodingCodes::BooleanFalse => Ok(false),
            _ => Err(Error::InvalidFormatCode)
        }
    }

    #[inline]
    fn parse_i8(&mut self) -> Result<i8, Error> {
        match self.read_format_code()? {
            EncodingCodes::Byte => {
                let byte = self.reader.next()?;
                Ok(byte as i8)
            },
            _ => Err(Error::InvalidFormatCode)
        }
    }

    #[inline]
    fn parse_i16(&mut self) -> Result<i16, Error> {
        match self.read_format_code()? {
            EncodingCodes::Short => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(i16::from_be_bytes(bytes))
            },
            _ => Err(Error::InvalidFormatCode)
        }
    }

    #[inline]
    fn parse_i32(&mut self) -> Result<i32, Error> {
        match self.read_format_code()? {
            EncodingCodes::Int => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(i32::from_be_bytes(bytes))
            },
            EncodingCodes::SmallInt => {
                let byte = self.reader.next()?;
                Ok(byte as i32)
            },
            _ => Err(Error::InvalidFormatCode)
        }
    }

    #[inline]
    fn parse_i64(&mut self) -> Result<i64, Error> {
        match self.read_format_code()? {
            EncodingCodes::Long => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(i64::from_be_bytes(bytes))
            },
            EncodingCodes::SmallLong => {
                let byte = self.reader.next()?;
                Ok(byte as i64)
            },
            _ => Err(Error::InvalidFormatCode)
        }
    }

    #[inline]
    fn parse_u8(&mut self) -> Result<u8, Error> {
        match self.read_format_code()? {
            EncodingCodes::Ubyte => {
                let byte = self.reader.next()?;
                Ok( byte )
            },
            _ => Err(Error::InvalidFormatCode)
        }
    }

    #[inline]
    fn parse_u16(&mut self) -> Result<u16, Error> {
        match self.read_format_code()? {
            EncodingCodes::Ushort => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(u16::from_be_bytes(bytes))
            },
            _ => Err(Error::InvalidFormatCode)
        }
    }

    #[inline]
    fn parse_u32(&mut self) -> Result<u32, Error> {
        match self.read_format_code()? {
            EncodingCodes::Uint => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(u32::from_be_bytes(bytes))
            },
            EncodingCodes::SmallUint => {
                let byte = self.reader.next()?;
                Ok(byte as u32)
            },
            EncodingCodes::Uint0 => {
                Ok(0)
            },
            _ => Err(Error::InvalidFormatCode)
        }
    }

    #[inline]
    fn parse_u64(&mut self) -> Result<u64, Error> {
        match self.read_format_code()? {
            EncodingCodes::Ulong => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(u64::from_be_bytes(bytes))
            },
            EncodingCodes::SmallUlong => {
                let byte = self.reader.next()?;
                Ok(byte as u64)
            },
            EncodingCodes::Ulong0 => {
                Ok(0)
            },
            _ => Err(Error::InvalidFormatCode)
        }
    }

    #[inline]
    fn parse_f32(&mut self) -> Result<f32, Error> {
        match self.read_format_code()? {
            EncodingCodes::Float => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(f32::from_be_bytes(bytes))
            },
            _ => Err(Error::InvalidFormatCode)
        }
    }

    #[inline]
    fn parse_f64(&mut self) -> Result<f64, Error> {
        match self.read_format_code()? {
            EncodingCodes::Double => {
                let bytes = self.reader.read_const_bytes()?;
                Ok(f64::from_be_bytes(bytes))
            },
            _ => Err(Error::InvalidFormatCode)
        }
    }

    #[inline]
    fn parse_char(&mut self) -> Result<char, Error> {
        match self.read_format_code()? {
            EncodingCodes::Char => {
                let bytes = self.reader.read_const_bytes()?;
                let n = u32::from_be_bytes(bytes);
                char::from_u32(n).ok_or(Error::InvalidValue)
            },
            _ => Err(Error::InvalidFormatCode)
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
        V: de::Visitor<'de> 
    {
        unimplemented!()
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    #[inline]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_i8(self.parse_i8()?)
    }

    #[inline]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_i16(self.parse_i16()?)
    }

    #[inline]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_i32(self.parse_i32()?)
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_i64(self.parse_i64()?)
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_u8(self.parse_u8()?)
    }

    #[inline]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_u16(self.parse_u16()?)
    }

    #[inline]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_u32(self.parse_u32()?)
    }

    #[inline]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_u64(self.parse_u64()?)
    }

    #[inline]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_f32(self.parse_f32()?)
    }

    #[inline]
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_f64(self.parse_f64()?)
    }

    #[inline]
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        visitor.visit_char(self.parse_char()?)
    }

    #[inline]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        // visitor.visit_str(self.parse_str()?)
        todo!()
    }

    #[inline]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        // visitor.visit_str(self.parse_str()?)
        todo!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()    
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> ,
    {
        todo!()    
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()    
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()
    }
    
    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()
    }
    
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()    
    }

    fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()
    }

    fn deserialize_enum<V>(self, name: &'static str, variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::ser::to_vec;

    use super::from_slice;

    #[test]
    fn test_deserialize_bool() {

    }

    #[test]
    fn test_deserialize_i8() {
        let orig = 7i8;
        let serialized = to_vec(&orig).unwrap();
        let recovered: i8 = from_slice(&serialized).unwrap();
        assert_eq!(orig, recovered);
    }

    fn test_deserialize_u8() {

    }
}