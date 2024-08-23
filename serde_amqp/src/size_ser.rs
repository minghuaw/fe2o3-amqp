//! Serializer that calculates the size of serialized data without actually allocating `Vec<u8>`

use serde::ser::{self, SerializeMap};

use crate::{
    Error,
    __constants::{
        ARRAY, DECIMAL128, DECIMAL32, DECIMAL64, DESCRIBED_BASIC, DESCRIBED_LIST, DESCRIBED_MAP,
        DESCRIPTOR, SYMBOL, SYMBOL_REF, TIMESTAMP, TRANSPARENT_VEC, UUID,
    },
    ser::{U32_MAX_MINUS_4, U8_MAX, U8_MAX_MINUS_1},
    util::{FieldRole, IsArrayElement, NewType, StructEncoding},
};

/// Obtain the serialized size without allocating `Vec<u8>`
pub fn serialized_size<T>(value: &T) -> Result<usize, Error>
where
    T: ser::Serialize + ?Sized,
{
    let mut serializer = SizeSerializer::new();
    value.serialize(&mut serializer)
}

/// Serializer that calculates the size of serialized data without actually allocating `Vec<u8>`
#[derive(Debug)]
pub struct SizeSerializer {
    /// How a struct should be encoded
    pub(crate) struct_encoding: Vec<StructEncoding>,
    pub(crate) new_type: NewType,
    pub(crate) is_array_element: IsArrayElement,
}

impl Default for SizeSerializer {
    fn default() -> Self {
        Self::new()
    }
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
            IsArrayElement::False | IsArrayElement::FirstElement => Ok(2),
            IsArrayElement::OtherElement => Ok(1),
        }
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        match self.is_array_element {
            IsArrayElement::False | IsArrayElement::FirstElement => Ok(3),
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
            _ => unreachable!("serialize_i64 is only used for Long and Timestamp"),
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
                    }
                    U8_MAX..=U32_MAX_MINUS_4 => {
                        self.new_type = NewType::None;
                        Ok(5 + v.len())
                    }
                    _ => Err(Error::too_long()),
                },
                NewType::None => match v.len() {
                    0..=U8_MAX_MINUS_1 => Ok(2 + v.len()),
                    U8_MAX..=U32_MAX_MINUS_4 => Ok(5 + v.len()),
                    _ => Err(Error::too_long()),
                },
                _ => unreachable!("serialize_str is only used for Symbol and String"),
            },
            IsArrayElement::FirstElement => match self.new_type {
                NewType::Symbol | NewType::SymbolRef | NewType::None => Ok(5 + v.len()),
                _ => unreachable!("serialize_str is only used for Symbol and String"),
            },
            IsArrayElement::OtherElement => match self.new_type {
                NewType::Symbol | NewType::SymbolRef | NewType::None => Ok(4 + v.len()),
                _ => unreachable!("serialize_str is only used for Symbol and String"),
            },
        }
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let l = v.len();
        match self.new_type {
            NewType::None => match self.is_array_element {
                IsArrayElement::False => match l {
                    0..=U8_MAX_MINUS_1 => Ok(2 + l),
                    U8_MAX..=U32_MAX_MINUS_4 => Ok(5 + l),
                    _ => Err(Error::too_long()),
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
            NewType::Timestamp
            | NewType::Array
            | NewType::Symbol
            | NewType::SymbolRef
            | NewType::TransparentVec => unreachable!("serialize_bytes is only used for Binary, Decimal32, Decimal64, Decimal128, and Uuid"),
        }
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(1)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
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

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
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

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize + ?Sized,
    {
        if name == DESCRIPTOR {
            value.serialize(self).map(|len| len + 1)
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

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize + ?Sized,
    {
        match self.se.new_type {
            NewType::None => {
                let mut serializer = SizeSerializer::new();
                self.cumulated_size += value.serialize(&mut serializer)?;
            }
            NewType::Array => {
                let mut serializer = SizeSerializer::new();
                match self.idx {
                    0 => serializer.is_array_element = IsArrayElement::FirstElement,
                    _ => serializer.is_array_element = IsArrayElement::OtherElement,
                }
                self.cumulated_size += value.serialize(&mut serializer)?;
            }
            NewType::TransparentVec => {
                let mut serializer = SizeSerializer::new();
                self.cumulated_size += value.serialize(&mut serializer)?;
            }
            NewType::Dec32
            | NewType::Dec64
            | NewType::Dec128
            | NewType::Symbol
            | NewType::SymbolRef
            | NewType::Timestamp
            | NewType::Uuid => unreachable!("SeqSerializer is only used for List, Array, and TransparentVec"),
        }

        self.idx += 1;
        Ok(())
    }

    fn end(self) -> Result<usize, Error> {
        match self.se.new_type {
            NewType::None => list_size(self.cumulated_size, &self.se.is_array_element)
                .map_err(|_| Error::too_long()),
            NewType::Array => array_size(self.cumulated_size, &self.se.is_array_element)
                .map_err(|_| Error::too_long()),
            NewType::TransparentVec => {
                transparent_vec_size(self.cumulated_size, &self.se.is_array_element)
                    .map_err(|_| Error::too_long())
            }
            NewType::Dec32
            | NewType::Dec64
            | NewType::Dec128
            | NewType::Symbol
            | NewType::SymbolRef
            | NewType::Timestamp
            | NewType::Uuid => unreachable!("SeqSerializer is only used for List, Array, and TransparentVec"),
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
        0..=U8_MAX_MINUS_1 => match is_array_element {
            IsArrayElement::False => 1 + 2 + len,
            IsArrayElement::FirstElement => 1 + 2 + len,
            IsArrayElement::OtherElement => 2 + len,
        },
        U8_MAX..=U32_MAX_MINUS_4 => match is_array_element {
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

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize + ?Sized,
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

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: serde::Serialize + ?Sized,
        V: serde::Serialize + ?Sized,
    {
        let mut serializer = SizeSerializer::new();
        self.cumulated_size += key.serialize(&mut serializer)?;
        let mut serializer = SizeSerializer::new();
        self.cumulated_size += value.serialize(&mut serializer)?;
        Ok(())
    }

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Error>
    where
        T: ser::Serialize + ?Sized,
    {
        let mut serializer = SizeSerializer::new();
        self.cumulated_size += key.serialize(&mut serializer)?;
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize + ?Sized,
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
        0..=U8_MAX_MINUS_1 => match is_array_element {
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

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize + ?Sized,
    {
        match self.field_role {
            FieldRole::Descriptor => {
                self.field_role = FieldRole::Fields;
                let mut serializer = SizeSerializer::new();
                self.cumulated_size += value.serialize(&mut serializer)?;
                Ok(())
            }
            FieldRole::Fields => match self.se.struct_encoding() {
                StructEncoding::None => {
                    let mut serializer = SizeSerializer::new();
                    serializer.is_array_element = self.se.is_array_element.clone();
                    self.cumulated_size += value.serialize(&mut serializer)?;
                    Ok(())
                }
                StructEncoding::DescribedList => {
                    let mut serializer = SizeSerializer::new();
                    serializer.is_array_element = self.se.is_array_element.clone();
                    self.cumulated_size += value.serialize(&mut serializer)?;
                    Ok(())
                }
                StructEncoding::DescribedBasic => {
                    let mut serializer = SizeSerializer::new();
                    self.cumulated_size += value.serialize(&mut serializer)?;
                    Ok(())
                }
                StructEncoding::DescribedMap => unreachable!("TupleStructSerializer is NOT used for DescribedMap"),
            },
        }
    }

    fn end(self) -> Result<usize, Error> {
        match self.se.struct_encoding() {
            StructEncoding::None => list_size(self.cumulated_size, &self.se.is_array_element)
                .map_err(|_| Error::too_long()),
            StructEncoding::DescribedList => {
                let _ = self.se.struct_encoding.pop();
                list_size(self.cumulated_size, &self.se.is_array_element)
                    .map_err(|_| Error::too_long())
            }
            StructEncoding::DescribedBasic => {
                let _ = self.se.struct_encoding.pop();
                Ok(self.cumulated_size)
            }
            StructEncoding::DescribedMap => unreachable!("TupleStructSerializer is NOT used for DescribedMap"),
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

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize + ?Sized,
    {
        use ser::Serialize;

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
                }
                StructEncoding::DescribedList => {
                    let mut serializer = SizeSerializer::described_list();
                    self.cumulated_size += value.serialize(&mut serializer)?;
                    Ok(())
                }
                StructEncoding::DescribedMap => {
                    let mut serializer = SizeSerializer::described_map();
                    self.cumulated_size += key.serialize(&mut serializer)?;
                    self.cumulated_size += value.serialize(&mut serializer)?;
                    Ok(())
                }
                StructEncoding::DescribedBasic => {
                    self.cumulated_size += value.serialize(&mut *self.se)?;
                    Ok(())
                }
            }
        }
    }

    fn end(self) -> Result<usize, Error> {
        match self.se.struct_encoding() {
            StructEncoding::None => list_size(self.cumulated_size, &self.se.is_array_element)
                .map_err(|_| Error::too_long()),
            StructEncoding::DescribedList => {
                let _ = self.se.struct_encoding.pop();
                list_size(self.cumulated_size, &self.se.is_array_element)
                    .map_err(|_| Error::too_long())
            }
            StructEncoding::DescribedMap => {
                let _ = self.se.struct_encoding.pop();
                map_size(self.cumulated_size, &self.se.is_array_element)
                    .map_err(|_| Error::too_long())
            }
            StructEncoding::DescribedBasic => {
                let _ = self.se.struct_encoding.pop();
                Ok(self.cumulated_size)
            }
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

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize + ?Sized,
    {
        self.cumulated_size += value.serialize(&mut *self.se)?;
        Ok(())
    }

    fn end(self) -> Result<usize, Error> {
        let mut serializer = SizeSerializer::new();
        let key_len = ser::Serialize::serialize(&self.variant_index, &mut serializer)?;
        let value_len = list_size(self.cumulated_size, &self.se.is_array_element)
            .map_err(|_| Error::too_long())?;

        map_size(key_len + value_len, &self.se.is_array_element).map_err(|_| Error::too_long())
    }
}

impl<'a> ser::SerializeStructVariant for VariantSerializer<'a> {
    type Ok = usize;
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ser::Serialize + ?Sized,
    {
        <Self as ser::SerializeTupleVariant>::serialize_field(self, value)
    }

    fn end(self) -> Result<usize, Error> {
        <Self as ser::SerializeTupleVariant>::end(self)
    }
}

#[allow(unused_imports, clippy::all)]
#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use serde_bytes::ByteBuf;

    use crate::{
        primitives::{Array, Dec128, Dec32, Dec64, Symbol, Timestamp, Uuid},
        to_vec,
    };

    use super::serialized_size;

    #[test]
    fn serialized_size_of_bool() {
        let value = false;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_u8() {
        let value = 0u8;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_u16() {
        let value = 0u16;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 255u16;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 256u16;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_u32() {
        let value = 0u32;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 255u32;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 256u32;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_u64() {
        let value = 0u64;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 255u64;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 256u64;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_i8() {
        let value = 0i8;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_i16() {
        let value = 0i16;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 255i16;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 256i16;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_i32() {
        let value = 0i32;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 127i32;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 128i32;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_i64() {
        let value = 0i64;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 127i64;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = 128i64;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_f32() {
        let value = 3.14f32;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_f64() {
        let value = 3.14f64;
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_dec32() {
        let value = Dec32::from([0, 1, 2, 3]);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_dec64() {
        let value = Dec64::from([0, 1, 2, 3, 4, 5, 6, 7]);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_dec128() {
        let value = Dec128::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_char() {
        let value = 'a';
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_timestamp() {
        let value = Timestamp::from(1234);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_uuid() {
        let value = Uuid::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_vbin() {
        let value = ByteBuf::from(vec![]);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = ByteBuf::from(vec![0; 255]);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = ByteBuf::from(vec![0; 256]);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = ByteBuf::from(vec![0; 65535]);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_str() {
        let value = "";
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = "a";
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = String::from_utf8(vec![b'a'; 255]).unwrap();
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = String::from_utf8(vec![b'a'; 256]).unwrap();
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_symbol() {
        let value = Symbol::from("");
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = Symbol::from("a");
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = Symbol::from(String::from_utf8(vec![b'a'; 255]).unwrap());
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = Symbol::from(String::from_utf8(vec![b'a'; 256]).unwrap());
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_list() {
        let value = Vec::<u8>::new();
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = vec![0u8; 127];
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = vec![0u8; 128];
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_map() {
        let mut value = HashMap::<u8, u8>::new();
        value.insert(1, 2);
        value.insert(3, 4);
        value.insert(5, 6);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_array() {
        let value = Array::from(vec![1u8]);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = Array::from(vec![0u8; 253]);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = Array::from(vec![0u8; 254]);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn serialized_size_of_derived_struct() {
        use crate as serde_amqp;
        use crate::macros::SerializeComposite;

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = "0x00:0x13", encoding = "list")]
        struct Foo {
            is_fool: bool,
            a: i32,
        }

        let value = Foo {
            is_fool: true,
            a: 9,
        };
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    #[allow(unused_macros)]
    fn serialized_size_of_derived_struct_map_encoding() {
        use crate as serde_amqp;
        use crate::macros::SerializeComposite;

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = "0x00:0x13", encoding = "map")]
        struct Foo {
            is_fool: bool,
            a: i32,
        }

        let value = Foo {
            is_fool: true,
            a: 9,
        };
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn serialized_size_of_derived_tuple_struct() {
        use crate as serde_amqp;
        use crate::macros::SerializeComposite;

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = "0x00:0x13", encoding = "list")]
        struct Foo(bool, i32);

        let value = Foo(true, 9);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn serialized_size_of_newtype_wrapper() {
        use crate as serde_amqp;
        use crate::macros::SerializeComposite;

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = "0x00:0x01", encoding = "basic")]
        struct Wrapper(BTreeMap<Symbol, i32>);

        let mut map = BTreeMap::new();
        map.insert(Symbol::from("a"), 1);
        map.insert(Symbol::from("b"), 2);
        let value = Wrapper(map);
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn serialized_size_of_struct_with_optional_field() {
        use crate as serde_amqp;
        use crate::macros::SerializeComposite;

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = "0x00:0x01", encoding = "list")]
        struct Foo {
            a: i32,
            b: Option<i32>,
        }

        let value = Foo { a: 1, b: None };
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());

        let value = Foo { a: 1, b: Some(2) };
        let ssize = serialized_size(&value).unwrap();
        let buf = to_vec(&value).unwrap();
        assert_eq!(ssize, buf.len());
    }

    #[test]
    fn serialized_size_of_described_list_of_timestamps() {
        use crate::{primitives::*, Value, described::Described, descriptor::Descriptor};

        let timestamp = Timestamp::from_milliseconds(12345);
        let mut list = List::new();
        list.push(Value::Timestamp(timestamp));
    
        let described = Described {
            descriptor: Descriptor::Code(0x73),
            value: Value::List(list),
        };
    
        let value = Value::Described(Box::new(described));
    
        let size_result = serialized_size(&value);
        assert!(size_result.is_ok());
    }
}
