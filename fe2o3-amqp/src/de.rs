use std::{convert::TryInto};
use serde::de;

use crate::{format_code::EncodingCodes, error::Error, format::{ArrayWidth, CompoundWidth, FixedWidth, VariableWidth}, read::{IoReader, Read}, unpack, unpack_or_eof};

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

    fn read_format_code(&mut self) -> Option<Result<EncodingCodes, Error>> {
        let code = self.reader.next();
        let code = unpack!(code);
        Some(code.try_into())
    }

    fn read_fixed_width_bytes(&mut self, width: FixedWidth) -> Option<Result<Vec<u8>, Error>> {
        let n = width as usize;
        self.reader.read_bytes(n)
    }

    fn read_variable_width_bytes(&mut self, width: VariableWidth) -> Option<Result<Vec<u8>, Error>> {
        let n = match width {
            VariableWidth::One => {
                let size = unpack!(self.reader.next());
                size as usize
            },
            VariableWidth::Four => {
                let size_buf = unpack!(self.reader.read_const_bytes::<4>());
                u32::from_be_bytes(size_buf) as usize
            }
        };

        self.reader.read_bytes(n)
    }

    fn read_compound_bytes(&mut self, width: CompoundWidth) -> Option<Result<CompoundBuf, Error>> {
        let cb = match width {
            CompoundWidth::Zero => {
                CompoundBuf { count: 0, value_buf: Vec::with_capacity(0) }
            },
            CompoundWidth::One => {
                let size = unpack!(self.reader.next());
                let count = unpack!(self.reader.next());

                // Need to subtract the one byte taken by `count`
                let size = size - 1;
                let value_buf = unpack!(self.reader.read_bytes(size as usize));
                CompoundBuf {count: count as u32, value_buf }
            },
            CompoundWidth::Four => {
                let size_buf = unpack!(self.reader.read_const_bytes::<4>());
                let count_buf = unpack!(self.reader.read_const_bytes::<4>());
                let size = u32::from_be_bytes(size_buf);
                let count = u32::from_be_bytes(count_buf);

                // Need to substract the four byte taken by `count`
                let size = size - 4;
                let value_buf = unpack!(self.reader.read_bytes(size as usize));
                CompoundBuf { count, value_buf }
            }
        };

        Some(Ok(cb))
    }

    fn read_array_bytes(&mut self, width: ArrayWidth) -> Option<Result<ArrayBuf, Error>> {
        let ab = match width {
            ArrayWidth::One => {
                let size = unpack!(self.reader.next());
                let count = unpack!(self.reader.next());
                let elem_code = unpack!(self.reader.next());

                // Must subtract the one byte taken by `count` and one byte taken by `elem_code`
                let size = size - 1 - 1;
                let value_buf = unpack!(self.reader.read_bytes(size as usize));
                ArrayBuf { count: count as u32, elem_code, value_buf }
            },
            ArrayWidth::Four => {
                let size_buf = unpack!(self.reader.read_const_bytes::<4>());
                let count_buf = unpack!(self.reader.read_const_bytes::<4>());
                let elem_code = unpack!(self.reader.next());

                let size = u32::from_be_bytes(size_buf);
                let count = u32::from_be_bytes(count_buf);

                // Must subtract the four bytes taken by `count` and one byte taken by `elem_code`
                let size = size - 4 - 1;
                let value_buf = unpack!(self.reader.read_bytes(size as usize));
                ArrayBuf { count, elem_code, value_buf }
            }
        };

        Some(Ok(ab))
    }

    #[inline]
    fn parse_bool(&mut self) -> Result<bool, Error> {
        todo!()
    }

    #[inline]
    fn parse_i8(&mut self) -> Result<i8, Error> {
        let byte = unpack_or_eof!(self.reader.next());
        Ok(byte as i8)
    }

    #[inline]
    fn parse_i16(&mut self) -> Result<i16, Error> {
        todo!()
    }

    #[inline]
    fn parse_i32(&mut self) -> Result<i32, Error> {
        todo!()
    }

    #[inline]
    fn parse_i64(&mut self) -> Result<i64, Error> {
        todo!()
    }

    #[inline]
    fn parse_u8(&mut self) -> Result<u8, Error> {
        let byte = unpack_or_eof!(self.reader.next());
        Ok( byte )
    }

    #[inline]
    fn parse_u16(&mut self) -> Result<u16, Error> {
        todo!()
    }

    #[inline]
    fn parse_u32(&mut self) -> Result<u32, Error> {
        todo!()
    }
}


impl<'de, 'a, R> de::Deserializer<'de> for &'a mut Deserializer<R> 
where
    R: Read<'de>,
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        unimplemented!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        unimplemented!()
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        match unpack_or_eof!(self.read_format_code()) {
            EncodingCodes::Byte => {
                visitor.visit_i8(self.parse_i8()?)
            },
            _ => Err(Error::InvalidFormatCode)
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: de::Visitor<'de> {
        todo!()
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: de::Visitor<'de> {
        todo!()
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        todo!()
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> 
    {
        match unpack_or_eof!(self.read_format_code()) {
            EncodingCodes::Ubyte => {
                visitor.visit_u8(self.parse_u8()?)
            }
            _ => Err(Error::InvalidFormatCode)
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: de::Visitor<'de> {
        todo!()
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: de::Visitor<'de> {
        todo!()
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: de::Visitor<'de> {
        todo!()
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: de::Visitor<'de> {
        todo!()
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: de::Visitor<'de> {
        todo!()
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: de::Visitor<'de> {
        todo!()
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: de::Visitor<'de> {
        todo!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
            V: de::Visitor<'de> {
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