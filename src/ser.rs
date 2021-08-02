use std::{io::Write, num::Wrapping};

use serde::{Serialize, ser};

use crate::{codes::EncodingCodes, error::{Error}};

pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, Error> 
where 
    T: Serialize
{
    let mut writer = Vec::new(); // TODO: pre-allocate capacity
    let mut serializer = Serializer::new(&mut writer);
    value.serialize(&mut serializer)?;
    Ok(writer)
}

pub struct Serializer<W> {
    writer: W
}

impl<W: Write> Serializer<W> {
    fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<'a, W: Write + 'a> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;
    
    type SerializeSeq = Compound<'a, W>;
    type SerializeMap = Compound<'a, W>;
    type SerializeStruct = Compound<'a, W>;
    type SerializeStructVariant = Compound<'a, W>;
    type SerializeTuple = Compound<'a, W>;
    type SerializeTupleStruct = Compound<'a, W>;
    type SerializeTupleVariant = Compound<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        match v {
            true => {
                let buf = [EncodingCodes::BooleanTrue as u8];
                self.writer.write_all(&buf)
            },
            false => {
                let buf = [EncodingCodes::BooleanFalse as u8];
                self.writer.write_all(&buf)
            }
        }
        .map_err(|err| err.into())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        let buf = [EncodingCodes::Ubyte as u8, v];
        self.writer.write_all(&buf)
            .map_err(|err| err.into())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        let code = [EncodingCodes::Ushort as u8];
        let buf: [u8; 2] = v.to_be_bytes();
        self.writer.write_all(&code)?;
        self.writer.write_all(&buf)
            .map_err(|err| err.into())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        match v {
            // uint0
            0 => {
                let buf = [EncodingCodes::Uint0 as u8];
                self.writer.write_all(&buf)
            },
            // smalluint
            val @ 1..=255 => {
                let buf = [EncodingCodes::SmallUint as u8, val as u8];
                self.writer.write_all(&buf)
            },
            // uint
            val @ _ => {
                let code = [EncodingCodes::Uint as u8];
                let buf: [u8; 4] = val.to_be_bytes();
                self.writer.write_all(&code)?;
                self.writer.write_all(&buf)
            }
        }
        .map_err(|err| err.into())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        match v {
            // ulong0
            0 => {
                let buf = [EncodingCodes::Ulong0 as u8];
                self.writer.write_all(&buf)
            },
            // small ulong
            val @ 1..=255 => {
                let buf = [EncodingCodes::SmallUlong as u8, val as u8];
                self.writer.write_all(&buf)
            },
            // ulong
            val @ _ => {
                let code = [EncodingCodes::Ulong as u8];
                let buf: [u8; 8] = val.to_be_bytes();
                self.writer.write_all(&code)?;
                self.writer.write_all(&buf)
            }
        }
        .map_err(|err| err.into())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
    
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        // None is serialized as Bson::Null in BSON
        unimplemented!()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
            T: Serialize 
    {
        // Some(T) is serialized simply as if it is T in BSON
        unimplemented!()
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        // unit is serialized as Bson::Null in BSON
        unimplemented!()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
            T: Serialize 
    {
        unimplemented!()
    }
    
    fn serialize_newtype_variant<T: ?Sized>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
            T: Serialize 
    {
        unimplemented!()
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> {
        unimplemented!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        unimplemented!()
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        unimplemented!()
    }

    fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant, Self::Error> {
        unimplemented!()
    }
}

// pub enum Compound<'a, W: 'a> {
//     Map { ser: &'a mut Serializer<W> },
    
// }

pub struct Compound<'a, W: 'a> {
    se: &'a mut Serializer<W>
}

impl<'a, W: Write + 'a> ser::SerializeSeq for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
            T: Serialize 
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, W: Write + 'a> ser::SerializeTuple for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
            T: Serialize 
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, W: Write + 'a> ser::SerializeTupleStruct for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
            T: Serialize 
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, W: Write + 'a> ser::SerializeTupleVariant for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
            T: Serialize 
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, W: Write + 'a> ser::SerializeMap for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
            T: Serialize 
    {
        unimplemented!()
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
            T: Serialize 
    {
        unimplemented!()
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
            K: Serialize,
            V: Serialize, 
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, W: Write + 'a> ser::SerializeStruct for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
            T: Serialize 
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, W: Write + 'a> ser::SerializeStructVariant for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
            T: Serialize 
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use crate::codes::EncodingCodes;

    use super::*;

    fn assert_eq_on_serialized_vs_expected<T: Serialize>(val: T, expected: Vec<u8>) {
        let serialized = to_vec(&val).unwrap();
        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_bool() {
        let val = true;
        let expected = vec![EncodingCodes::BooleanTrue as u8];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = false;
        let expected = vec![EncodingCodes::BooleanFalse as u8];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_i8() {
        unimplemented!();
        let val = 3i8;
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_i16() {
        unimplemented!();
        let val = 3i16;
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_i32() {
        unimplemented!();
        // small int
        let val = 3i32;
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        // int
        let val = 300i32;
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_i64() {
        unimplemented!();
        // small long
        let val = 3i64;
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        // long
        let val = 300i64;
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_u8() {
        let val = u8::MIN;
        let expected = vec![EncodingCodes::Ubyte as u8, u8::MIN];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = u8::MAX;
        let expected = vec![EncodingCodes::Ubyte as u8, u8::MAX];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_u16() {
        let val = u16::MIN;
        let mut expected = vec![EncodingCodes::Ushort as u8];
        expected.append(&mut Vec::from(val.to_be_bytes()));
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = 131u16;
        let mut expected = vec![EncodingCodes::Ushort as u8];
        expected.append(&mut Vec::from(val.to_be_bytes()));
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = u16::MAX;
        let mut expected = vec![EncodingCodes::Ushort as u8];
        expected.append(&mut Vec::from(val.to_be_bytes()));
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_u32() {
        // uint0
        let val = 0u32;
        let expected = vec![EncodingCodes::Uint0 as u8];
        assert_eq_on_serialized_vs_expected(val, expected);

        // small uint
        let val = 255u32;
        let expected = vec![EncodingCodes::SmallUint as u8, 255];
        assert_eq_on_serialized_vs_expected(val, expected);

        // uint
        let val = u32::MAX;
        let mut expected = vec![EncodingCodes::Uint as u8];
        expected.append(&mut val.to_be_bytes().into());
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_u64() {
        // ulong0
        let val = 0u64;
        let expected = vec![EncodingCodes::Ulong0 as u8];
        assert_eq_on_serialized_vs_expected(val, expected);

        // small ulong
        let val = 255u64;
        let expected = vec![EncodingCodes::SmallUlong as u8, 255];
        assert_eq_on_serialized_vs_expected(val, expected);

        // ulong
        let val = u64::MAX;
        let mut expected = vec![EncodingCodes::Ulong as u8];
        expected.append(&mut val.to_be_bytes().into());
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_f32() {
        unimplemented!();

        let val = 5.0f32;
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_f64() {
        unimplemented!();

        let val = 5.0f64;
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_char() {
        unimplemented!();

        let val = 'c';
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_str() {
        unimplemented!();
        
        let val = "true";
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_bytes() {
        unimplemented!();
        let val = vec![1,2,3];
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_none() {
        unimplemented!();
        let val: Option<()> = None;
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_some() {
        unimplemented!();
        let val = Some(1);
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_unit() {
        unimplemented!();
        let val = ();
        let expected = vec![0u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);
    }
}