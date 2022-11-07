//! Serializer that calculates the size of serialized data without actually allocating `Vec<u8>`

use serde::ser::{self, SerializeMap};

use crate::{Error, util::{IsArrayElement, NewType, StructEncoding, FieldRole}, ser::{U8_MAX_MINUS_1, U32_MAX_MINUS_4, U8_MAX_MINUS_2, U8_MAX, U32_MAX_MINUS_8}, __constants::{SYMBOL, SYMBOL_REF, ARRAY, DECIMAL32, DECIMAL64, DECIMAL128, TIMESTAMP, UUID, TRANSPARENT_VEC, DESCRIPTOR, DESCRIBED_BASIC, DESCRIBED_LIST, DESCRIBED_MAP}};

/// Serializer that calculates the size of serialized data without actually allocating `Vec<u8>`
#[derive(Debug)]
pub struct SizeSerializer { 
    /// How a struct should be encoded
    pub(crate) struct_encoding: Vec<StructEncoding>,
    pub(crate) new_type: NewType,
    pub(crate) is_array_element: IsArrayElement,
}

impl SizeSerializer {
    /// Create a new `SizeSerializer`
    pub fn new() -> Self {
        Self {
            struct_encoding: Vec::new(),
            new_type: NewType::None,
            is_array_element: IsArrayElement::False,
        }
    }

    fn described_list() -> Self {
        Self {
            struct_encoding: vec![StructEncoding::DescribedList],
            new_type: NewType::None,
            is_array_element: IsArrayElement::False,
        }
    }

    fn described_map() -> Self {
        Self {
            struct_encoding: vec![StructEncoding::DescribedMap],
            new_type: NewType::None,
            is_array_element: IsArrayElement::False,
        }
    }

    // fn described_basic() -> Self {
    //     Self {
    //         struct_encoding: vec![StructEncoding::DescribedBasic],
    //         new_type: NewType::None,
    //         is_array_element: IsArrayElement::False,
    //     }
    // }

    fn struct_encoding(&self) -> &StructEncoding {
        self.struct_encoding.last().unwrap_or(&StructEncoding::None)
    }
}

impl<'a> ser::Serializer for &'a mut SizeSerializer {
    type Ok = usize;

    type Error = Error;

    type SerializeSeq = SeqSerializer<'a>;

    type SerializeTuple = TupleSerializer<'a>;

    type SerializeTupleStruct = TupleStructSerializer<'a>;

    type SerializeTupleVariant = VariantSerializer<'a>;

    type SerializeMap = MapSerializer<'a>;

    type SerializeStruct = StructSerializer<'a>;

    type SerializeStructVariant = VariantSerializer<'a>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False => Ok(1),
            IsArrayElement::FirstElement => Ok(2),
            IsArrayElement::OtherElement => Ok(1),
        }
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False | 
            IsArrayElement::FirstElement => Ok(2),
            IsArrayElement::OtherElement => Ok(1),
        }
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False | 
            IsArrayElement::FirstElement => Ok(3),
            IsArrayElement::OtherElement => Ok(2),
        }
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False => match v {
                -128..=127 => Ok(2),
                _ => Ok(5),
            },
            IsArrayElement::FirstElement => Ok(5),
            IsArrayElement::OtherElement => Ok(4),
        }
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        match self.new_type {
            NewType::None => match self.is_array_element {
                IsArrayElement::False => match v {
                    -128..=127 => Ok(2),
                    _ => Ok(9),
                },
                IsArrayElement::FirstElement => Ok(9),
                IsArrayElement::OtherElement => Ok(8),
            },
            NewType::Timestamp => match self.is_array_element {
                IsArrayElement::False => Ok(9),
                IsArrayElement::FirstElement => Ok(9),
                IsArrayElement::OtherElement => Ok(8),
            },
            _ => unreachable!()
        }
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False => Ok(2),
            IsArrayElement::FirstElement => Ok(2),
            IsArrayElement::OtherElement => Ok(1),
        }
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False => Ok(3),
            IsArrayElement::FirstElement => Ok(3),
            IsArrayElement::OtherElement => Ok(2),
        }
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False => match v {
                0 => Ok(1),
                1..=255 => Ok(2),
                _ => Ok(5),
            },
            IsArrayElement::FirstElement => Ok(5),
            IsArrayElement::OtherElement => Ok(4),
        }
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False => match v {
                0 => Ok(1),
                1..=255 => Ok(2),
                _ => Ok(9),
            },
            IsArrayElement::FirstElement => Ok(9),
            IsArrayElement::OtherElement => Ok(8),
        }
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False => Ok(5),
            IsArrayElement::FirstElement => Ok(5),
            IsArrayElement::OtherElement => Ok(4),
        }
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False => Ok(9),
            IsArrayElement::FirstElement => Ok(9),
            IsArrayElement::OtherElement => Ok(8),
        }
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False => Ok(5),
            IsArrayElement::FirstElement => Ok(5),
            IsArrayElement::OtherElement => Ok(4),
        }
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False => match self.new_type {
                NewType::Symbol | NewType::SymbolRef => match v.len() {
                    0..=U8_MAX_MINUS_1 => {
                        self.new_type = NewType::None;
                        Ok(2 + v.len())
                    },
                    256..=U32_MAX_MINUS_4 => {
                        self.new_type = NewType::None;
                        Ok(5 + v.len())
                    },
                    _ => Err(Error::too_long())
                },
                NewType::None => match v.len() {
                    0..=U8_MAX_MINUS_1 => Ok(2 + v.len()),
                    256..=U32_MAX_MINUS_4 => Ok(5 + v.len()),
                    _ => Err(Error::too_long())
                },
                _ => unreachable!()
            },
            IsArrayElement::FirstElement => match self.new_type {
                NewType::Symbol | NewType::SymbolRef | NewType::None => Ok(5 + v.len()),
                _ => unreachable!()
            },
            IsArrayElement::OtherElement => match self.new_type {
                NewType::Symbol | NewType::SymbolRef | NewType::None => Ok(4 + v.len()),
                _ => unreachable!()
            },
        }
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let l = v.len();
        match self.new_type {
            NewType::None => match self.is_array_element {
                IsArrayElement::False => match l {
                    0..=U8_MAX_MINUS_1 => Ok(2 + l),
                    256..=U32_MAX_MINUS_4 => Ok(5 + l),
                    _ => Err(Error::too_long())
                },
                IsArrayElement::FirstElement => Ok(5 + l),
                IsArrayElement::OtherElement => Ok(4 + l),
            },
            NewType::Dec32 => match self.is_array_element {
                IsArrayElement::False => Ok(1 + l),
                IsArrayElement::FirstElement => Ok(1 + l),
                IsArrayElement::OtherElement => Ok(l),
            },
            NewType::Dec64 => match self.is_array_element {
                IsArrayElement::False => Ok(1 + l),
                IsArrayElement::FirstElement => Ok(1 + l),
                IsArrayElement::OtherElement => Ok(l),
            },
            NewType::Dec128 => match self.is_array_element {
                IsArrayElement::False => Ok(1 + l),
                IsArrayElement::FirstElement => Ok(1 + l),
                IsArrayElement::OtherElement => Ok(l),
            },
            NewType::Uuid => match self.is_array_element {
                IsArrayElement::False => Ok(1 + l),
                IsArrayElement::FirstElement => Ok(1 + l),
                IsArrayElement::OtherElement => Ok(l),
            },
            NewType::Timestamp |
            NewType::Array |
            NewType::Symbol |
            NewType::SymbolRef |
            NewType::TransparentVec => unreachable!(),
        }
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(1)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(1)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_u32(variant_index)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize {
        if name == SYMBOL {
            self.new_type = NewType::Symbol;
        } else if name == SYMBOL_REF {
            self.new_type = NewType::SymbolRef;
        } else if name == ARRAY {
            self.new_type = NewType::Array;
        } else if name == DECIMAL32 {
            self.new_type = NewType::Dec32;
        } else if name == DECIMAL64 {
            self.new_type = NewType::Dec64;
        } else if name == DECIMAL128 {
            self.new_type = NewType::Dec128;
        } else if name == TIMESTAMP {
            self.new_type = NewType::Timestamp
        } else if name == UUID {
            self.new_type = NewType::Uuid;
        } else if name == TRANSPARENT_VEC {
            self.new_type = NewType::TransparentVec;
        }
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize {
        if name == DESCRIPTOR {
            value.serialize(self).map(|len| len+1)
        } else {
            let mut state = self.serialize_map(Some(1))?;
            state.serialize_entry(&variant_index, value)?;
            state.end()
        }
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer::new(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(TupleSerializer::new(self))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        if name == DESCRIBED_BASIC {
            self.struct_encoding.push(StructEncoding::DescribedBasic);
            Ok(TupleStructSerializer::descriptor(self))
        } else if name == DESCRIBED_LIST {
            self.struct_encoding.push(StructEncoding::DescribedList);
            Ok(TupleStructSerializer::descriptor(self))
        } else {
            Ok(TupleStructSerializer::fields(self))
        }
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(VariantSerializer::new(variant_index, self))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer::new(self))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        if name == DESCRIBED_LIST {
            self.struct_encoding.push(StructEncoding::DescribedList);
            Ok(StructSerializer::new(self))
        } else if name == DESCRIBED_BASIC {
            self.struct_encoding.push(StructEncoding::DescribedBasic);
            Ok(StructSerializer::new(self))
        } else if name == DESCRIBED_MAP {
            self.struct_encoding.push(StructEncoding::DescribedMap);
            Ok(StructSerializer::new(self))
        } else {
            Ok(StructSerializer::new(self))
        }
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(VariantSerializer::new(variant_index, self))
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
}


/// SeqSerializer that calculates the size of serialized data without actually allocating `Vec<u8>`
#[derive(Debug)]
pub struct SeqSerializer<'a> { 
    cumulated_size: usize,
    idx: usize,
    se: &'a mut SizeSerializer,
}

impl<'a> SeqSerializer<'a> {
    fn new(se: &'a mut SizeSerializer) -> Self {
        Self {
            cumulated_size: 0,
            idx: 0,
            se,
        }
    }
}

impl<'a> ser::SerializeSeq for SeqSerializer<'a> {
    type Ok = usize;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize,
    {
        match self.se.new_type {
            NewType::None => {
                self.cumulated_size += value.serialize(&mut *self.se)?;
            },
            NewType::Array => {
                let mut serializer = SizeSerializer::new();
                match self.idx {
                    0 => serializer.is_array_element = IsArrayElement::FirstElement,
                    _ => serializer.is_array_element = IsArrayElement::OtherElement,
                }
                self.cumulated_size += value.serialize(&mut serializer)?;
            },
            NewType::TransparentVec => {
                let mut serializer = SizeSerializer::new();
                self.cumulated_size += value.serialize(&mut serializer)?;
            },
            NewType::Dec32 |
            NewType::Dec64 |
            NewType::Dec128 |
            NewType::Symbol |
            NewType::SymbolRef |
            NewType::Timestamp |
            NewType::Uuid => unreachable!(),
        }

        self.idx += 1;
        Ok(())
    }

    fn end(self) -> Result<usize, Error> {
        match self.se.new_type {
            NewType::None => list_size(self.cumulated_size, &self.se.is_array_element).map_err(|_| Error::too_long()),
            NewType::Array => array_size(self.cumulated_size, &self.se.is_array_element).map_err(|_| Error::too_long()),
            NewType::TransparentVec => transparent_vec_size(self.cumulated_size, &self.se.is_array_element).map_err(|_| Error::too_long()),
            NewType::Dec32 |
            NewType::Dec64 |
            NewType::Dec128 |
            NewType::Symbol |
            NewType::SymbolRef |
            NewType::Timestamp |
            NewType::Uuid => unreachable!(),
        }
    }
}

fn list_size(len: usize, is_array_element: &IsArrayElement) -> Result<usize, usize> {
    match len {
        0 => Ok(1),
        1..=U8_MAX_MINUS_1 => match is_array_element {
            IsArrayElement::False => Ok(1 + 2 + len),
            IsArrayElement::FirstElement => Ok(1 + 2 + len),
            IsArrayElement::OtherElement => Ok(2 + len),
        },
        U8_MAX..=U32_MAX_MINUS_4 => match is_array_element {
            IsArrayElement::False => Ok(1 + 4 + 4 + len),
            IsArrayElement::FirstElement => Ok(1 + 4 + 4 + len),
            IsArrayElement::OtherElement => Ok(4 + 4 + len),
        },
        _ => Err(len),
    }
}

fn array_size(len: usize, is_array_element: &IsArrayElement) -> Result<usize, usize> {
    let out = match len {
        0..=U8_MAX_MINUS_2 => match is_array_element {
            IsArrayElement::False => 1 + 2 + len,
            IsArrayElement::FirstElement => 1 + 2 + len,
            IsArrayElement::OtherElement => 2 + len,
        },
        U8_MAX_MINUS_1..=U32_MAX_MINUS_4 => match is_array_element {
            IsArrayElement::False => 1 + 4 + 4 + len,
            IsArrayElement::FirstElement => 1 + 4 + 4 + len,
            IsArrayElement::OtherElement => 4 + 4 + len,
        },
        _ => return Err(len),
    };
    Ok(out)
}

fn transparent_vec_size(len: usize, _is_array_element: &IsArrayElement) -> Result<usize, usize> {
    Ok(len)
}

/// SeqSerializer that calculates the size of serialized data without actually allocating `Vec<u8>`
#[derive(Debug)]
pub struct TupleSerializer<'a> { 
    cumulated_size: usize,
    se: &'a mut SizeSerializer,
}

impl<'a> TupleSerializer<'a> {
    fn new(se: &'a mut SizeSerializer) -> Self {
        Self {
            cumulated_size: 0,
            se,
        }
    }
}

impl<'a> ser::SerializeTuple for TupleSerializer<'a> {
    type Ok = usize;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize,
    {
        let mut serializer = SizeSerializer::new();
        self.cumulated_size += value.serialize(&mut serializer)?;
        Ok(())
    }

    fn end(self) -> Result<usize, Error> {
        list_size(self.cumulated_size, &self.se.is_array_element).map_err(|_| Error::too_long())
    }
}

/// MapSerializer that calculates the size of serialized data without actually allocating `Vec<u8>`
#[derive(Debug)]
pub struct MapSerializer<'a> {
    cumulated_size: usize,
    se: &'a mut SizeSerializer,
}

impl<'a> MapSerializer<'a> {
    fn new(se: &'a mut SizeSerializer) -> Self {
        Self {
            cumulated_size: 0,
            se,
        }
    }
}

impl<'a> ser::SerializeMap for MapSerializer<'a> {
    type Ok = usize;
    type Error = Error;

    fn serialize_entry<K: ?Sized, V: ?Sized>(
            &mut self,
            key: &K,
            value: &V,
        ) -> Result<(), Self::Error>
        where
            K: serde::Serialize,
            V: serde::Serialize, {
        let mut serializer = SizeSerializer::new();
        self.cumulated_size += key.serialize(&mut serializer)?;
        let mut serializer = SizeSerializer::new();
        self.cumulated_size += value.serialize(&mut serializer)?;
        Ok(())
    }

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Error>
    where
        T: ser::Serialize,
    {
        let mut serializer = SizeSerializer::new();
        self.cumulated_size += key.serialize(&mut serializer)?;
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize,
    {
        let mut serializer = SizeSerializer::new();
        self.cumulated_size += value.serialize(&mut serializer)?;
        Ok(())
    }

    fn end(self) -> Result<usize, Error> {
        map_size(self.cumulated_size, &self.se.is_array_element).map_err(|_| Error::too_long())
    }
}

fn map_size(len: usize, is_array_element: &IsArrayElement) -> Result<usize, usize> {
    match len {
        0..=U8_MAX_MINUS_2 => match is_array_element {
            IsArrayElement::False => Ok(1 + 2 + len),
            IsArrayElement::FirstElement => Ok(1 + 2 + len),
            IsArrayElement::OtherElement => Ok(2 + len),
        },
        U8_MAX_MINUS_1..=U32_MAX_MINUS_8 => match is_array_element {
            IsArrayElement::False => Ok(1 + 4 + 4 + len),
            IsArrayElement::FirstElement => Ok(1 + 4 + 4 + len),
            IsArrayElement::OtherElement => Ok(4 + 4 + len),
        },
        _ => Err(len),
    }
}

/// SeqSerializer that calculates the size of serialized data without actually allocating `Vec<u8>`
#[derive(Debug)]
pub struct TupleStructSerializer<'a> { 
    field_role: FieldRole,
    cumulated_size: usize,
    se: &'a mut SizeSerializer,
}

impl<'a> TupleStructSerializer<'a> {
    fn descriptor(se: &'a mut SizeSerializer) -> Self {
        Self {
            cumulated_size: 0,
            field_role: FieldRole::Descriptor,
            se,
        }
    }

    fn fields(se: &'a mut SizeSerializer) -> Self {
        Self {
            cumulated_size: 0,
            field_role: FieldRole::Fields,
            se,
        }
    }
}

impl<'a> ser::SerializeTupleStruct for TupleStructSerializer<'a> {
    type Ok = usize;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize,
    {
        match self.field_role {
            FieldRole::Descriptor => {
                self.field_role = FieldRole::Fields;
                let mut serializer = SizeSerializer::new();
                self.cumulated_size += value.serialize(&mut serializer)?;
                Ok(())
            },
            FieldRole::Fields => match self.se.struct_encoding() {
                StructEncoding::None => {
                    let mut serializer = SizeSerializer::new();
                    serializer.is_array_element = self.se.is_array_element.clone();
                    self.cumulated_size += value.serialize(&mut serializer)?;
                    Ok(())
                },
                StructEncoding::DescribedList => {
                    let mut serializer = SizeSerializer::new();
                    serializer.is_array_element = self.se.is_array_element.clone();
                    self.cumulated_size += value.serialize(&mut serializer)?;
                    Ok(())
                },
                StructEncoding::DescribedBasic => {
                    let mut serializer = SizeSerializer::new();
                    self.cumulated_size += value.serialize(&mut serializer)?;
                    Ok(())
                },
                StructEncoding::DescribedMap => unreachable!(),
            }
        }
    }

    fn end(self) -> Result<usize, Error> {
        match self.se.struct_encoding() {
            StructEncoding::None => list_size(self.cumulated_size, &self.se.is_array_element).map_err(|_| Error::too_long()),
            StructEncoding::DescribedList => {
                let _ = self.se.struct_encoding.pop();
                list_size(self.cumulated_size, &self.se.is_array_element).map_err(|_| Error::too_long())
            },
            StructEncoding::DescribedBasic => {
                let _ = self.se.struct_encoding.pop();
                Ok(self.cumulated_size)
            },
            StructEncoding::DescribedMap => unreachable!(),
        }
    }
}

/// SeqSerializer that calculates the size of serialized data without actually allocating `Vec<u8>`
#[derive(Debug)]
pub struct StructSerializer<'a> { 
    cumulated_size: usize,
    se: &'a mut SizeSerializer,
}

impl<'a> StructSerializer<'a> {
    fn new(se: &'a mut SizeSerializer) -> Self {
        Self {
            cumulated_size: 0,
            se,
        }
    }
}

impl<'a> ser::SerializeStruct for StructSerializer<'a> {
    type Ok = usize;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize,
    {
        if key == DESCRIPTOR {
            self.cumulated_size += value.serialize(&mut *self.se)?;
            Ok(())
        } else {
            match self.se.struct_encoding() {
                StructEncoding::None => {
                    let mut serializer = SizeSerializer::new();
                    serializer.is_array_element = self.se.is_array_element.clone();
                    self.cumulated_size += value.serialize(&mut serializer)?;
                    Ok(())
                },
                StructEncoding::DescribedList => {
                    let mut serializer = SizeSerializer::described_list();
                    self.cumulated_size += value.serialize(&mut serializer)?;
                    Ok(())
                },
                StructEncoding::DescribedMap => {
                    let mut serializer = SizeSerializer::described_map();
                    self.cumulated_size += value.serialize(&mut serializer)?;
                    Ok(())
                },
                StructEncoding::DescribedBasic => {
                    self.cumulated_size += value.serialize(&mut *self.se)?;
                    Ok(())
                },
            }
        }
    }

    fn end(self) -> Result<usize, Error> {
        match self.se.struct_encoding() {
            StructEncoding::None => list_size(self.cumulated_size, &self.se.is_array_element).map_err(|_| Error::too_long()),
            StructEncoding::DescribedList => {
                let _ = self.se.struct_encoding.pop();
                list_size(self.cumulated_size, &self.se.is_array_element).map_err(|_| Error::too_long())
            },
            StructEncoding::DescribedMap => {
                let _ = self.se.struct_encoding.pop();
                map_size(self.cumulated_size, &self.se.is_array_element).map_err(|_| Error::too_long())
            },
            StructEncoding::DescribedBasic => {
                let _ = self.se.struct_encoding.pop();
                Ok(self.cumulated_size)
            },
        }
    }
}

/// Serializer that calculates the size of serialized data without actually allocating `Vec<u8>`
#[derive(Debug)]
pub struct VariantSerializer<'a> {
    cumulated_size: usize,
    variant_index: u32,
    se: &'a mut SizeSerializer,
}

impl<'a> VariantSerializer<'a> {
    fn new(variant_index: u32, se: &'a mut SizeSerializer) -> Self {
        Self {
            cumulated_size: 0,
            se,
            variant_index,
        }
    }
}

impl<'a> ser::SerializeTupleVariant for VariantSerializer<'a> {
    type Ok = usize;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize,
    {
        self.cumulated_size += value.serialize(&mut *self.se)?;
        Ok(())
    }

    fn end(self) -> Result<usize, Error> {
        let mut serializer = SizeSerializer::new();
        let key_len = ser::Serialize::serialize(&self.variant_index, &mut serializer)?;
        let value_len = list_size(self.cumulated_size, &self.se.is_array_element).map_err(|_| Error::too_long())?;

        map_size(key_len + value_len, &self.se.is_array_element).map_err(|_| Error::too_long())
    }
}

impl<'a> ser::SerializeStructVariant for VariantSerializer<'a> {
    type Ok = usize;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize,
    {
        <Self as ser::SerializeTupleVariant>::serialize_field(self, value)
    }

    fn end(self) -> Result<usize, Error> {
        <Self as ser::SerializeTupleVariant>::end(self)
    }
}