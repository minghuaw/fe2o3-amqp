//! Serializer implementation

use std::io::Write;

use bytes::BufMut;
use serde::{
    ser::{self, SerializeMap},
    Serialize,
};

use crate::{
    __constants::{
        ARRAY, DECIMAL128, DECIMAL32, DECIMAL64, DESCRIBED_BASIC, DESCRIBED_LIST, DESCRIBED_MAP,
        DESCRIPTOR, SYMBOL, TIMESTAMP, UUID,
    },
    error::Error,
    format::{OFFSET_LIST32, OFFSET_LIST8, OFFSET_MAP32, OFFSET_MAP8},
    format_code::EncodingCodes,
    util::{FieldRole, IsArrayElement, NewType, StructEncoding},
};

// Variable type will spend a byte on size
const U8_MAX_MINUS_1: usize = u8::MAX as usize - 1;
const U8_MAX_MINUS_2: usize = u8::MAX as usize - 2;

// Variable type will spend 4 bytes on size
const U32_MAX_MINUS_4: usize = u32::MAX as usize - 4;
const U32_MAX_MINUS_8: usize = u32::MAX as usize - 8;

/// Serializes the given value into a byte vector
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, Error>
where
    T: Serialize,
{
    let mut writer = Vec::new();
    let mut serializer = Serializer::new(&mut writer);
    value.serialize(&mut serializer)?;
    Ok(writer)
}

/// A struct for serializing Rust structs/values into AMQP1.0 wire format
#[derive(Debug)]
pub struct Serializer<W> {
    /// The output of serialized data
    pub writer: W,

    /// Any particular new_type wrapper
    new_type: NewType,

    /// How a struct should be encoded
    struct_encoding: Vec<StructEncoding>,

    /// Whether we are serializing an array
    /// NOTE: This should only be changed by `SeqSerializer`
    pub is_array_elem: IsArrayElement,
}

impl<W: Write> From<W> for Serializer<W> {
    fn from(writer: W) -> Self {
        Self::new(writer)
    }
}

impl<W: Write> Serializer<W> {
    /// Creates a new AMQP1.0 serializer
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            new_type: Default::default(),
            struct_encoding: Default::default(),
            is_array_elem: IsArrayElement::False,
        }
    }

    /// Consume the serializer and obtain the inner writer
    pub fn into_inner(self) -> W {
        self.writer
    }

    /// Creates an AMQP1.0 serializer for Symbol type
    pub fn symbol(writer: W) -> Self {
        Self {
            writer,
            new_type: NewType::Symbol,
            struct_encoding: Default::default(),
            is_array_elem: IsArrayElement::False,
        }
    }

    /// Creates an AMQP1.0 serializer for described list
    pub fn described_list(writer: W) -> Self {
        Self {
            writer,
            new_type: Default::default(),
            struct_encoding: vec![StructEncoding::DescribedList],
            is_array_elem: IsArrayElement::False,
        }
    }

    /// Creates an AMQP1.0 serilaizer for described map
    pub fn described_map(writer: W) -> Self {
        Self {
            writer,
            new_type: Default::default(),
            struct_encoding: vec![StructEncoding::DescribedMap],
            is_array_elem: IsArrayElement::False,
        }
    }

    /// Creates an AMQP1.0 serializer for a described wrapper
    pub fn described_basic(writer: W) -> Self {
        Self {
            writer,
            new_type: Default::default(),
            struct_encoding: vec![StructEncoding::DescribedBasic],
            is_array_elem: IsArrayElement::False,
        }
    }

    fn struct_encoding(&self) -> &StructEncoding {
        self.struct_encoding
            .last()
            .unwrap_or_else(|| &StructEncoding::None)
    }
}

impl<'a, W: Write + 'a> ser::Serializer for &'a mut Serializer<W> {
    // A separate serializer is used for intermediate representation
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a, W>;
    type SerializeTuple = TupleSerializer<'a, W>;
    type SerializeMap = MapSerializer<'a, W>;
    type SerializeTupleStruct = TupleStructSerializer<'a, W>;
    type SerializeStruct = StructSerializer<'a, W>;
    type SerializeTupleVariant = VariantSerializer<'a, W>;

    // The variant struct will be serialized as Value in DescribedSerializer
    type SerializeStructVariant = VariantSerializer<'a, W>;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        match self.is_array_elem {
            IsArrayElement::False => {
                let buf = match v {
                    true => [EncodingCodes::BooleanTrue as u8],
                    false => [EncodingCodes::BooleanFalse as u8],
                };
                // This cannot be moved out of match because array have different length
                self.writer.write_all(&buf)
            }
            IsArrayElement::FirstElement => {
                let buf = match v {
                    true => [EncodingCodes::Boolean as u8, 0x01],
                    false => [EncodingCodes::Boolean as u8, 0x00],
                };
                self.writer.write_all(&buf)
            }
            IsArrayElement::OtherElement => {
                let buf = match v {
                    true => [0x01u8],
                    false => [0x00u8],
                };
                self.writer.write_all(&buf)
            }
        }
        .map_err(Into::into)
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        match self.is_array_elem {
            IsArrayElement::False | IsArrayElement::FirstElement => {
                let buf = [EncodingCodes::Byte as u8, v as u8];
                self.writer.write_all(&buf).map_err(Into::into)
            }
            IsArrayElement::OtherElement => {
                let buf = [v as u8];
                self.writer.write_all(&buf).map_err(Into::into)
            }
        }
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        if let IsArrayElement::False | IsArrayElement::FirstElement = self.is_array_elem {
            let code = [EncodingCodes::Short as u8];
            self.writer.write_all(&code)?;
        }
        let buf = v.to_be_bytes();
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        match self.is_array_elem {
            IsArrayElement::False => match v {
                val @ -128..=127 => {
                    let buf = [EncodingCodes::SmallInt as u8, val as u8];
                    self.writer.write_all(&buf)?;
                }
                val @ _ => {
                    let code = [EncodingCodes::Int as u8];
                    self.writer.write_all(&code)?;
                    let buf: [u8; 4] = val.to_be_bytes();
                    self.writer.write_all(&buf)?;
                }
            },
            IsArrayElement::FirstElement => {
                let code = [EncodingCodes::Int as u8];
                self.writer.write_all(&code)?;
                let buf: [u8; 4] = v.to_be_bytes();
                self.writer.write_all(&buf)?;
            }
            IsArrayElement::OtherElement => {
                let buf: [u8; 4] = v.to_be_bytes();
                self.writer.write_all(&buf)?;
            }
        }
        Ok(())
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        match self.new_type {
            NewType::None => match self.is_array_elem {
                IsArrayElement::False => match v {
                    val @ -128..=127 => {
                        let buf = [EncodingCodes::SmallLong as u8, val as u8];
                        self.writer.write_all(&buf)?;
                    }
                    val @ _ => {
                        let code = [EncodingCodes::Long as u8];
                        self.writer.write_all(&code)?;
                        let buf: [u8; 8] = val.to_be_bytes();
                        self.writer.write_all(&buf)?;
                    }
                },
                IsArrayElement::FirstElement => {
                    let code = [EncodingCodes::Long as u8];
                    self.writer.write_all(&code)?;
                    let buf: [u8; 8] = v.to_be_bytes();
                    self.writer.write_all(&buf)?;
                }
                IsArrayElement::OtherElement => {
                    let buf: [u8; 8] = v.to_be_bytes();
                    self.writer.write_all(&buf)?;
                }
            },
            NewType::Timestamp => {
                if let IsArrayElement::False | IsArrayElement::FirstElement = self.is_array_elem {
                    let code = [EncodingCodes::Timestamp as u8];
                    self.writer.write_all(&code)?;
                }
                let buf = v.to_be_bytes();
                self.writer.write_all(&buf)?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        match self.is_array_elem {
            IsArrayElement::False | IsArrayElement::FirstElement => {
                let buf = [EncodingCodes::UByte as u8, v];
                self.writer.write_all(&buf)?;
            }
            IsArrayElement::OtherElement => {
                let buf = [v];
                self.writer.write_all(&buf)?;
            }
        }
        Ok(())
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        if let IsArrayElement::False | IsArrayElement::FirstElement = self.is_array_elem {
            let code = [EncodingCodes::UShort as u8];
            self.writer.write_all(&code)?;
        }
        let buf: [u8; 2] = v.to_be_bytes();
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        match self.is_array_elem {
            IsArrayElement::False => {
                match v {
                    // uint0
                    0 => {
                        let buf = [EncodingCodes::Uint0 as u8];
                        self.writer.write_all(&buf)?;
                    }
                    // smalluint
                    val @ 1..=255 => {
                        let buf = [EncodingCodes::SmallUint as u8, val as u8];
                        self.writer.write_all(&buf)?;
                    }
                    // uint
                    val @ _ => {
                        let code = [EncodingCodes::UInt as u8];
                        self.writer.write_all(&code)?;
                        let buf: [u8; 4] = val.to_be_bytes();
                        self.writer.write_all(&buf)?;
                    }
                }
            }
            IsArrayElement::FirstElement => {
                let code = [EncodingCodes::UInt as u8];
                self.writer.write_all(&code)?;
                let buf: [u8; 4] = v.to_be_bytes();
                self.writer.write_all(&buf)?;
            }
            IsArrayElement::OtherElement => {
                let buf: [u8; 4] = v.to_be_bytes();
                self.writer.write_all(&buf)?;
            }
        }
        Ok(())
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        match self.is_array_elem {
            IsArrayElement::False => {
                match v {
                    // ulong0
                    0 => {
                        let buf = [EncodingCodes::Ulong0 as u8];
                        self.writer.write_all(&buf)?;
                    }
                    // small ulong
                    val @ 1..=255 => {
                        let buf = [EncodingCodes::SmallUlong as u8, val as u8];
                        self.writer.write_all(&buf)?;
                    }
                    // ulong
                    val @ _ => {
                        let code = [EncodingCodes::ULong as u8];
                        self.writer.write_all(&code)?;
                        let buf: [u8; 8] = val.to_be_bytes();
                        self.writer.write_all(&buf)?;
                    }
                }
            }
            IsArrayElement::FirstElement => {
                let code = [EncodingCodes::ULong as u8];
                self.writer.write_all(&code)?;
                let buf: [u8; 8] = v.to_be_bytes();
                self.writer.write_all(&buf)?;
            }
            IsArrayElement::OtherElement => {
                let buf: [u8; 8] = v.to_be_bytes();
                self.writer.write_all(&buf)?;
            }
        }
        Ok(())
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        if let IsArrayElement::False | IsArrayElement::FirstElement = self.is_array_elem {
            let code = [EncodingCodes::Float as u8];
            self.writer.write_all(&code)?;
        }
        let buf = v.to_be_bytes();
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        if let IsArrayElement::False | IsArrayElement::FirstElement = self.is_array_elem {
            let code = [EncodingCodes::Double as u8];
            self.writer.write_all(&code)?;
        }
        let buf = v.to_be_bytes();
        self.writer.write_all(&buf).map_err(Into::into)
    }

    // `char` in rust is a subset of the unicode code points and
    // can be directly treated as u32
    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        if let IsArrayElement::False | IsArrayElement::FirstElement = self.is_array_elem {
            let code = [EncodingCodes::Char as u8];
            self.writer.write_all(&code)?;
        }
        let buf = (v as u32).to_be_bytes();
        self.writer.write_all(&buf).map_err(Into::into)
    }

    // String slices are always valid utf-8
    // `String` is utf-8 encoded
    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        match self.is_array_elem {
            IsArrayElement::False => {
                match self.new_type {
                    NewType::Symbol => {
                        // Symbols are encoded as ASCII characters [ASCII].
                        //
                        // Returns the length of this String, in bytes,
                        // not chars or graphemes. In other words, it might
                        // not be what a human considers the length of the string.
                        let l = v.len();

                        match l {
                            // sym8
                            0..=U8_MAX_MINUS_1 => {
                                let code = [EncodingCodes::Sym8 as u8, l as u8];
                                self.writer.write_all(&code)?;
                            }
                            256..=U32_MAX_MINUS_4 => {
                                let code = [EncodingCodes::Sym32 as u8];
                                let width = (l as u32).to_be_bytes();
                                self.writer.write_all(&code)?;
                                self.writer.write_all(&width)?;
                            }
                            _ => return Err(Error::too_long()),
                        }
                        self.new_type = NewType::None;
                    }
                    NewType::None => {
                        // A string represents a sequence of Unicode characters
                        // as defined by the Unicode V6.0.0 standard [UNICODE6].
                        let l = v.chars().count();
                        match l {
                            // str8-utf8
                            0..=U8_MAX_MINUS_1 => {
                                let code = [EncodingCodes::Str8 as u8, l as u8];
                                // let width: [u8; 1] = (l as u8).to_be_bytes();
                                self.writer.write_all(&code)?;
                                // self.writer.write_all(&width)?;
                            }
                            // str32-utf8
                            256..=U32_MAX_MINUS_4 => {
                                let code = [EncodingCodes::Str32 as u8];
                                let width: [u8; 4] = (l as u32).to_be_bytes();
                                self.writer.write_all(&code)?;
                                self.writer.write_all(&width)?;
                            }
                            _ => return Err(Error::too_long()),
                        }
                    }
                    _ => unreachable!(),
                }
            }
            IsArrayElement::FirstElement => match self.new_type {
                NewType::Symbol => {
                    // Symbols are encoded as ASCII characters [ASCII].
                    //
                    // Returns the length of this String, in bytes,
                    // not chars or graphemes. In other words, it might
                    // not be what a human considers the length of the string.
                    let l = v.len();

                    let code = [EncodingCodes::Sym32 as u8];
                    let width = (l as u32).to_be_bytes();
                    self.writer.write_all(&code)?;
                    self.writer.write_all(&width)?;
                }
                NewType::None => {
                    // A string represents a sequence of Unicode characters
                    // as defined by the Unicode V6.0.0 standard [UNICODE6].
                    let l = v.chars().count();

                    let code = [EncodingCodes::Str32 as u8];
                    let width: [u8; 4] = (l as u32).to_be_bytes();
                    self.writer.write_all(&code)?;
                    self.writer.write_all(&width)?;
                }
                _ => unreachable!(),
            },
            IsArrayElement::OtherElement => match self.new_type {
                NewType::Symbol => {
                    // Symbols are encoded as ASCII characters [ASCII].
                    //
                    // Returns the length of this String, in bytes,
                    // not chars or graphemes. In other words, it might
                    // not be what a human considers the length of the string.
                    let l = v.len();

                    let width = (l as u32).to_be_bytes();
                    self.writer.write_all(&width)?;
                }
                NewType::None => {
                    // A string represents a sequence of Unicode characters
                    // as defined by the Unicode V6.0.0 standard [UNICODE6].
                    let l = v.chars().count();

                    let width: [u8; 4] = (l as u32).to_be_bytes();
                    self.writer.write_all(&width)?;
                }
                _ => unreachable!(),
            },
        }

        self.writer.write_all(v.as_bytes()).map_err(Into::into)
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let l = v.len();
        match self.new_type {
            NewType::None => {
                match self.is_array_elem {
                    IsArrayElement::False => {
                        match l {
                            // vbin8
                            0..=U8_MAX_MINUS_1 => {
                                let code = [EncodingCodes::VBin8 as u8];
                                let width: [u8; 1] = (l as u8).to_be_bytes();
                                self.writer.write_all(&code)?;
                                self.writer.write_all(&width)?;
                            }
                            // vbin32
                            256..=U32_MAX_MINUS_4 => {
                                let code = [EncodingCodes::VBin32 as u8];
                                let width: [u8; 4] = (l as u32).to_be_bytes();
                                self.writer.write_all(&code)?;
                                self.writer.write_all(&width)?;
                            }
                            _ => return Err(Error::too_long()),
                        }
                    }
                    IsArrayElement::FirstElement => {
                        let code = [EncodingCodes::VBin32 as u8];
                        let width: [u8; 4] = (l as u32).to_be_bytes();
                        self.writer.write_all(&code)?;
                        self.writer.write_all(&width)?;
                    }
                    IsArrayElement::OtherElement => {
                        let width: [u8; 4] = (l as u32).to_be_bytes();
                        self.writer.write_all(&width)?;
                    }
                }
            }
            NewType::Dec32 => {
                if let IsArrayElement::False | IsArrayElement::FirstElement = self.is_array_elem {
                    let code = [EncodingCodes::Decimal32 as u8];
                    self.writer.write_all(&code)?;
                }
                self.new_type = NewType::None;
            }
            NewType::Dec64 => {
                if let IsArrayElement::False | IsArrayElement::FirstElement = self.is_array_elem {
                    let code = [EncodingCodes::Decimal64 as u8];
                    self.writer.write_all(&code)?;
                }
                self.new_type = NewType::None;
            }
            NewType::Dec128 => {
                if let IsArrayElement::False | IsArrayElement::FirstElement = self.is_array_elem {
                    let code = [EncodingCodes::Decimal128 as u8];
                    self.writer.write_all(&code)?;
                }
                self.new_type = NewType::None;
            }
            NewType::Uuid => {
                if let IsArrayElement::False | IsArrayElement::FirstElement = self.is_array_elem {
                    let code = [EncodingCodes::Uuid as u8];
                    self.writer.write_all(&code)?;
                }
                self.new_type = NewType::None;
            }
            // Timestamp should be handled by i64
            NewType::Timestamp => unreachable!(),
            NewType::Array => unreachable!(),
            NewType::Symbol => unreachable!(),
        }

        self.writer.write_all(v).map_err(Into::into)
    }

    // None is serialized as Bson::Null in BSON
    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        let buf = [EncodingCodes::Null as u8];
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        // Some(T) is serialized simply as if it is T in BSON
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        // unit is serialized as Bson::Null in BSON
        let buf = [EncodingCodes::Null as u8];
        self.writer.write_all(&buf).map_err(Into::into)
    }

    // JSON, BSOM, AVRO all serialized to unit
    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    // Serialize into tag
    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_u32(variant_index)
    }

    // Treat new_type structs as insignificant wrappers around the data they contain
    //
    // Serialization of `Symbol`
    // - The serializer will determine whether name == SYMBOL_MAGIC
    // - If a Symbol is to be serialized, it will write a Symbol constructor and modify it's internal state to Symbol
    #[inline]
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        if name == SYMBOL {
            self.new_type = NewType::Symbol;
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
            self.new_type = NewType::Uuid
        }
        value.serialize(self)
    }

    // Treat new_type variant as insignificant wrappers around the data they contain
    #[inline]
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        // use ser::SerializeSeq;
        if name == DESCRIPTOR
        // || name == VALUE || name == AMQP_ERROR || name == CONNECTION_ERROR || name == SESSION_ERROR || name == LINK_ERROR
        {
            let code = [EncodingCodes::DescribedType as u8];
            self.writer.write_all(&code)?;
            value.serialize(self)
        } else {
            let mut state = self.serialize_map(Some(1))?;
            state.serialize_entry(&variant_index, value)?;
            // state.serialize_ele(value)?;
            state.end()
        }
    }

    // A variably sized heterogeneous sequence of values
    //
    // This will be encoded as primitive type `Array`
    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        // The most external array should be treated as IsArrayElement::False
        Ok(SeqSerializer::new(self))
    }

    // A statically sized heterogeneous sequence of values
    // for which the length will be known at deserialization
    // time without looking at the serialized data
    //
    // This will be encoded as primitive type `List`
    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(TupleSerializer::new(self, len))
    }

    // A named tuple, for example `struct Rgb(u8, u8, u8)`
    //
    // The tuple struct looks rather like a rust-exclusive data type.
    // Thus this will be treated the same as a tuple (as in JSON and AVRO)
    #[inline]
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        // This will never be called on a Described type because the Decribed type is
        // a struct.
        // Simply serialize the content as seq
        if name == DESCRIBED_BASIC {
            self.struct_encoding.push(StructEncoding::DescribedBasic);
            // let code = [EncodingCodes::DescribedType as u8];
            // self.writer.write_all(&code)?;
            Ok(TupleStructSerializer::descriptor(self))
        } else if name == DESCRIBED_LIST {
            self.struct_encoding.push(StructEncoding::DescribedList);
            // let code = [EncodingCodes::DescribedType as u8];
            // self.writer.write_all(&code)?;
            Ok(TupleStructSerializer::descriptor(self))
        } else {
            Ok(TupleStructSerializer::fields(self))
        }
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer::new(self))
    }

    // The serde data model treats struct as "A statically sized heterogeneous key-value pairing"
    //
    // Naive impl: serialize into a list (because Composite types in AMQP is serialized as
    // a described list)
    //
    // Can be configured to serialize into a map or list when wrapped with `Described`
    #[inline]
    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        // The name should override the parent struct encoding
        let result = if name == DESCRIBED_LIST {
            self.struct_encoding.push(StructEncoding::DescribedList);
            Ok(StructSerializer::list_value(self))
        } else if name == DESCRIBED_MAP {
            self.struct_encoding.push(StructEncoding::DescribedMap);
            Ok(StructSerializer::map_value(self))
        } else if name == DESCRIBED_BASIC {
            self.struct_encoding.push(StructEncoding::DescribedBasic);
            Ok(StructSerializer::basic_value(self))
        } else {
            match self.struct_encoding() {
                // A None state indicates a freshly instantiated serializer
                StructEncoding::None => {
                    // Only non-described struct will go to this branch
                    Ok(StructSerializer::list_value(self))
                }
                StructEncoding::DescribedBasic => Ok(StructSerializer::basic_value(self)),
                StructEncoding::DescribedList => Ok(StructSerializer::list_value(self)),
                StructEncoding::DescribedMap => Ok(StructSerializer::map_value(self)),
            }
        };
        result
    }

    // Treat this as if it is a tuple because this kind of enum is unique in rust
    #[inline]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(VariantSerializer::new(
            self,
            name,
            variant_index,
            variant,
            len,
        ))
    }

    // Treat it as if it is a struct because this is not found in other languages
    #[inline]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(VariantSerializer::new(
            self,
            name,
            variant_index,
            variant,
            len,
        ))
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Serializer for sequence types
#[derive(Debug)]
pub struct SeqSerializer<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    num: usize,
    buf: Vec<u8>,
}

impl<'a, W: 'a> SeqSerializer<'a, W> {
    fn new(se: &'a mut Serializer<W>) -> Self {
        Self {
            se,
            num: 0,
            buf: Vec::new(),
        }
    }
}

// This requires some hacking way of getting the constructor (EncodingCode)
// for the type. Use TypeId?
//
// Serialize into a List
// List requires knowing the total number of bytes after serialized
impl<'a, W: Write + 'a> ser::SerializeSeq for SeqSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let mut se = match self.se.new_type {
            NewType::None => {
                // Element in the list always has it own constructor
                Serializer::new(&mut self.buf)
            }
            NewType::Array => {
                match self.num {
                    // The first element should include the contructor code
                    0 => {
                        let mut serializer = Serializer::new(&mut self.buf);
                        serializer.is_array_elem = IsArrayElement::FirstElement;
                        serializer
                    }
                    // The remaining element should only write the value bytes
                    _ => {
                        let mut serializer = Serializer::new(&mut self.buf);
                        serializer.is_array_elem = IsArrayElement::OtherElement;
                        serializer
                    }
                }
            }
            _ => unreachable!(),
        };

        self.num = self.num + 1;
        value.serialize(&mut se)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        let Self { se, num, buf } = self;
        match se.new_type {
            NewType::None => write_list(&mut se.writer, num, &buf, &se.is_array_elem),
            NewType::Array => write_array(&mut se.writer, num, &buf, &se.is_array_elem),
            _ => unreachable!(),
        }
    }
}

fn write_array<'a, W: Write + 'a>(
    mut writer: W,
    num: usize,
    buf: &'a [u8],
    ext_is_array_elem: &IsArrayElement,
) -> Result<(), Error> {
    let len = buf.len();

    match len {
        0..=U8_MAX_MINUS_2 => {
            if let IsArrayElement::False | IsArrayElement::FirstElement = ext_is_array_elem {
                let code = [EncodingCodes::Array8 as u8];
                writer.write_all(&code)?;
            }
            // `len` must include the one byte taken by `num`
            let len = len + 1; // not using const OFFSET because it includes format code
            let len_num = [len as u8, num as u8];
            writer.write_all(&len_num)?;
        }
        256..=U32_MAX_MINUS_8 => {
            if let IsArrayElement::False | IsArrayElement::FirstElement = ext_is_array_elem {
                let code = [EncodingCodes::Array32 as u8];
                writer.write_all(&code)?;
            }
            // `len` must include the four bytes taken by `num`
            let len = len + 4; // not using const OFFSET because it includes format code
            let len = (len as u32).to_be_bytes();
            let num = (num as u32).to_be_bytes();
            writer.write_all(&len)?;
            writer.write_all(&num)?;
        }
        _ => return Err(Error::too_long()),
    }
    writer.write_all(buf)?;
    Ok(())
}

/// Serializer for tuple types
#[derive(Debug)]
pub struct TupleSerializer<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    num: usize,
    buf: Vec<u8>,
}

impl<'a, W: 'a> TupleSerializer<'a, W> {
    fn new(se: &'a mut Serializer<W>, num: usize) -> Self {
        Self {
            se,
            num,
            buf: Vec::new(),
        }
    }
}

// Serialize into a List
// List requires knowing the total number of bytes after serialized
impl<'a, W: Write + 'a> ser::SerializeTuple for TupleSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let mut serializer = Serializer::new(&mut self.buf);
        value.serialize(&mut serializer)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        let Self { se, num, buf } = self;
        write_list(&mut se.writer, num, &buf, &se.is_array_elem)
    }
}

fn write_list<'a, W: Write + 'a>(
    mut writer: W,
    num: usize,
    buf: &'a [u8],
    ext_is_array_elem: &IsArrayElement,
) -> Result<(), Error> {
    let len = buf.len();

    // if `len` < 255, `num` must be smaller than 255
    match len {
        0 => {
            let code = [EncodingCodes::List0 as u8];
            writer.write_all(&code)?;
        }
        // FIXME: whether `len` should be below 255-1
        1..=U8_MAX_MINUS_1 => {
            if let IsArrayElement::False | IsArrayElement::FirstElement = ext_is_array_elem {
                let code = [EncodingCodes::List8 as u8];
                writer.write_all(&code)?;
            }
            // `len` must include the one byte taken by `num`
            let len = len + OFFSET_LIST8;
            let len_num = [len as u8, num as u8];
            writer.write_all(&len_num)?;
        }
        // FIXME: whether `len` should be below u32::MAX - 4
        256..=U32_MAX_MINUS_4 => {
            if let IsArrayElement::False | IsArrayElement::FirstElement = ext_is_array_elem {
                let code = [EncodingCodes::List32 as u8];
                writer.write_all(&code)?;
            }
            // Length including the four bytes taken by `num`
            let len = len + OFFSET_LIST32;
            let len: [u8; 4] = (len as u32).to_be_bytes();
            let num: [u8; 4] = (num as u32).to_be_bytes();
            writer.write_all(&len)?;
            writer.write_all(&num)?;
        }
        _ => return Err(Error::too_long()),
    }
    writer.write_all(buf)?;
    Ok(())
}

/// Serializer for map types
#[derive(Debug)]
pub struct MapSerializer<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    num: usize,
    buf: Vec<u8>,
}

impl<'a, W: 'a> MapSerializer<'a, W> {
    fn new(se: &'a mut Serializer<W>) -> Self {
        Self {
            se,
            num: 0,
            buf: Vec::new(),
        }
    }
}

// Map is a compound type and thus requires knowing the size of the total number
// of bytes
impl<'a, W: Write + 'a> ser::SerializeMap for MapSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: Serialize,
        V: Serialize,
    {
        let mut serializer = Serializer::new(&mut self.buf);
        key.serialize(&mut serializer)?;
        value.serialize(&mut serializer)?;
        self.num += 2;
        Ok(())
    }

    #[inline]
    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let mut serializer = Serializer::new(&mut self.buf);
        key.serialize(&mut serializer)?;
        self.num += 1;
        Ok(())
    }

    #[inline]
    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let mut serializer = Serializer::new(&mut self.buf);
        value.serialize(&mut serializer)?;
        self.num += 1;
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        let Self { se, num, buf } = self;
        write_map(&mut se.writer, num, &buf, &se.is_array_elem)
    }
}

fn write_map<'a, W: Write + 'a>(
    mut writer: W,
    num: usize,
    buf: &'a [u8],
    ext_is_array_elem: &IsArrayElement,
) -> Result<(), Error> {
    let len = buf.len();

    match len {
        // FIXME: Whether `len` should be 255 - 1
        0..=U8_MAX_MINUS_2 => {
            if let IsArrayElement::False | IsArrayElement::FirstElement = ext_is_array_elem {
                let code = [EncodingCodes::Map8 as u8];
                writer.write_all(&code)?;
            }
            // `len` must include the one byte taken by `num`
            let len = len + OFFSET_MAP8;
            let len_num = [len as u8, num as u8];
            writer.write_all(&len_num)?;
        }
        // FIXME: whether `len` should be u32::MAX - 4
        256..=U32_MAX_MINUS_8 => {
            if let IsArrayElement::False | IsArrayElement::FirstElement = ext_is_array_elem {
                let code = [EncodingCodes::Map32 as u8];
                writer.write_all(&code)?;
            }
            // `len` must include the four bytes taken by `num`
            let len = len + OFFSET_MAP32;
            let len = (len as u32).to_be_bytes();
            let num = (num as u32).to_be_bytes();
            writer.write_all(&len)?;
            writer.write_all(&num)?;
        }
        _ => return Err(Error::too_long()),
    }
    writer.write_all(buf)?;
    Ok(())
}

// Serialize into a List with
impl<'a, W: Write + 'a> ser::SerializeTupleStruct for TupleSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        <Self as ser::SerializeTuple>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as ser::SerializeTuple>::end(self)
    }
}

/// Serializer for tuple struct types
#[derive(Debug)]
pub struct TupleStructSerializer<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    field_role: FieldRole,
    count: usize,
    buf: Vec<u8>,
}

impl<'a, W: 'a> TupleStructSerializer<'a, W> {
    fn descriptor(se: &'a mut Serializer<W>) -> Self {
        Self {
            se,
            field_role: FieldRole::Descriptor,
            count: 0,
            buf: Vec::new(),
        }
    }

    fn fields(se: &'a mut Serializer<W>) -> Self {
        Self {
            se,
            field_role: FieldRole::Fields,
            count: 0,
            buf: Vec::new(),
        }
    }
}

impl<'a, W: 'a> AsMut<Serializer<W>> for TupleStructSerializer<'a, W> {
    fn as_mut(&mut self) -> &mut Serializer<W> {
        self.se
    }
}

impl<'a, W: Write + 'a> ser::SerializeTupleStruct for TupleStructSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        match self.field_role {
            FieldRole::Descriptor => {
                self.field_role = FieldRole::Fields;
                value.serialize(self.as_mut())
            }
            FieldRole::Fields => {
                self.count += 1;
                match self.se.struct_encoding() {
                    StructEncoding::None => {
                        // serialize regualr tuple struct as a list like in tuple
                        let mut serializer = Serializer::new(&mut self.buf);
                        serializer.is_array_elem = self.se.is_array_elem.clone();
                        value.serialize(&mut serializer)
                    }
                    StructEncoding::DescribedBasic => {
                        // simply serialize the value without buffering
                        value.serialize(self.as_mut())
                    }
                    StructEncoding::DescribedList => {
                        let mut serializer = Serializer::described_list(&mut self.buf);
                        value.serialize(&mut serializer)
                    }
                    StructEncoding::DescribedMap => {
                        unreachable!()
                    }
                }
            }
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self.se.struct_encoding() {
            StructEncoding::None => {
                // serialize regualr tuple struct as a list like in tuple
                write_list(
                    &mut self.se.writer,
                    self.count,
                    &self.buf,
                    &IsArrayElement::False,
                )
            }
            StructEncoding::DescribedBasic => {
                self.se.struct_encoding.pop();
                // simply serialize the value without buffering
                Ok(())
            }
            StructEncoding::DescribedList => {
                self.se.struct_encoding.pop();
                write_list(
                    &mut self.se.writer,
                    self.count,
                    &self.buf,
                    &IsArrayElement::False,
                )
            }
            StructEncoding::DescribedMap => {
                self.se.struct_encoding.pop();
                unreachable!()
            }
        }
    }
}

/// A serializer for struct types
#[derive(Debug)]
pub struct StructSerializer<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    count: usize,
    buf: Vec<u8>,
}

impl<'a, W: 'a> StructSerializer<'a, W> {
    fn basic_value(se: &'a mut Serializer<W>) -> Self {
        Self {
            se,
            count: 0,
            buf: vec![],
        }
    }

    fn list_value(se: &'a mut Serializer<W>) -> Self {
        // let buf = init_vec(&role);
        Self {
            se,
            count: 0,
            buf: vec![],
        }
    }

    fn map_value(se: &'a mut Serializer<W>) -> Self {
        // let buf = init_vec(&role);
        Self {
            se,
            count: 0,
            buf: vec![],
        }
    }
}

impl<'a, W: 'a> AsMut<Serializer<W>> for StructSerializer<'a, W> {
    fn as_mut(&mut self) -> &mut Serializer<W> {
        self.se
    }
}

// Serialize into a list
impl<'a, W: Write + 'a> ser::SerializeStruct for StructSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if key == DESCRIPTOR {
            value.serialize(self.as_mut())
        } else {
            self.count += 1;
            match self.se.struct_encoding() {
                StructEncoding::None => {
                    // normal struct will be serialized as a list
                    let mut serializer = Serializer::new(&mut self.buf);
                    serializer.is_array_elem = self.se.is_array_elem.clone();
                    value.serialize(&mut serializer)
                }
                StructEncoding::DescribedBasic => value.serialize(self.as_mut()),
                StructEncoding::DescribedList => {
                    let mut serializer = Serializer::described_list(&mut self.buf);
                    value.serialize(&mut serializer)
                }
                StructEncoding::DescribedMap => {
                    let mut serializer = Serializer::described_map(&mut self.buf);
                    key.serialize(&mut serializer)?;
                    value.serialize(&mut serializer)
                }
            }
        }
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self.se.struct_encoding() {
            StructEncoding::None => write_list(
                &mut self.se.writer,
                self.count,
                &self.buf,
                &self.se.is_array_elem,
            ),
            StructEncoding::DescribedBasic => {
                self.se.struct_encoding.pop();
                Ok(())
            }
            // The wrapper of value is always the `Described` struct. `Described` constructor is handled elsewhere
            StructEncoding::DescribedList => {
                self.se.struct_encoding.pop();
                write_list(
                    &mut self.se.writer,
                    self.count,
                    &self.buf,
                    &self.se.is_array_elem,
                )
            }
            // The wrapper of value is always the `Described` struct. `Described` constructor is handled elsewhere
            StructEncoding::DescribedMap => {
                self.se.struct_encoding.pop();
                write_map(
                    &mut self.se.writer,
                    self.count * 2,
                    &self.buf,
                    &self.se.is_array_elem,
                )
            }
        }
    }
}

/// Serializer for enum variants
#[derive(Debug)]
pub struct VariantSerializer<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    _name: &'static str,
    variant_index: u32,
    _variant: &'static str,
    num: usize,
    buf: Vec<u8>,
}

impl<'a, W: 'a> VariantSerializer<'a, W> {
    fn new(
        se: &'a mut Serializer<W>,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        num: usize, // number of field in the tuple
    ) -> Self {
        Self {
            se: se,
            _name: name,
            variant_index: variant_index,
            _variant: variant,
            num: num,
            buf: Vec::new(),
        }
    }
}

impl<'a, W: Write + 'a> ser::SerializeTupleVariant for VariantSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let mut se = Serializer::new(&mut self.buf);
        value.serialize(&mut se)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        use bytes::BytesMut;
        let kv_buf = BytesMut::new();
        let mut writer = kv_buf.writer();

        // Serialize key
        let mut key_se = Serializer::new(&mut writer);
        ser::Serialize::serialize(&self.variant_index, &mut key_se)?;

        // Write values
        write_list(&mut writer, self.num, &self.buf, &self.se.is_array_elem)?;

        // Write entire list
        let buf = writer.into_inner().freeze();
        // write_list(&mut self.se.writer, 2, &buf, &self.se.is_array_elem)
        write_map(&mut self.se.writer, 2, &buf, &self.se.is_array_elem)
    }
}

impl<'a, W: Write + 'a> ser::SerializeStructVariant for VariantSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        <Self as ser::SerializeTupleVariant>::serialize_field(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as ser::SerializeTupleVariant>::end(self)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        descriptor::Descriptor,
        format_code::EncodingCodes,
        primitives::{Array, Dec128, Dec32, Dec64, Symbol, Timestamp, Uuid},
    };

    use super::*;

    fn assert_eq_on_serialized_vs_expected<T: Serialize>(val: T, expected: &[u8]) {
        let serialized = to_vec(&val).unwrap();
        assert_eq!(&serialized[..], expected);
    }

    #[test]
    fn test_bool() {
        let val = true;
        let expected = vec![EncodingCodes::BooleanTrue as u8];
        assert_eq_on_serialized_vs_expected(val, &expected);

        let val = false;
        let expected = vec![EncodingCodes::BooleanFalse as u8];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_i8() {
        let val = 0i8;
        let expected = vec![EncodingCodes::Byte as u8, 0];
        assert_eq_on_serialized_vs_expected(val, &expected);

        let val = i8::MIN;
        let expected = vec![EncodingCodes::Byte as u8, 128u8];
        assert_eq_on_serialized_vs_expected(val, &expected);

        let val = i8::MAX;
        let expected = vec![EncodingCodes::Byte as u8, 127u8];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_i16() {
        let val = 0i16;
        let expected = vec![EncodingCodes::Short as u8, 0, 0];
        assert_eq_on_serialized_vs_expected(val, &expected);

        let val = -1i16;
        let expected = vec![EncodingCodes::Short as u8, 255, 255];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_i32() {
        // small int
        let val = 0i32;
        let expected = vec![EncodingCodes::SmallInt as u8, 0];
        assert_eq_on_serialized_vs_expected(val, &expected);

        // int
        let val = i32::MAX;
        let expected = vec![EncodingCodes::Int as u8, 127, 255, 255, 255];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_i64() {
        // small long
        let val = 0i64;
        let expected = vec![EncodingCodes::SmallLong as u8, 0];
        assert_eq_on_serialized_vs_expected(val, &expected);

        // long
        let val = i64::MAX;
        let expected = vec![
            EncodingCodes::Long as u8,
            127,
            255,
            255,
            255,
            255,
            255,
            255,
            255,
        ];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_u8() {
        let val = u8::MIN;
        let expected = vec![EncodingCodes::UByte as u8, 0];
        assert_eq_on_serialized_vs_expected(val, &expected);

        let val = u8::MAX;
        let expected = vec![EncodingCodes::UByte as u8, 255];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_u16() {
        let val = 0u16;
        let expected = vec![EncodingCodes::UShort as u8, 0, 0];
        assert_eq_on_serialized_vs_expected(val, &expected);

        let val = 131u16;
        let expected = vec![EncodingCodes::UShort as u8, 0, 131];
        assert_eq_on_serialized_vs_expected(val, &expected);

        let val = 65535u16;
        let expected = vec![EncodingCodes::UShort as u8, 255, 255];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_u32() {
        // uint0
        let val = 0u32;
        let expected = vec![EncodingCodes::Uint0 as u8];
        assert_eq_on_serialized_vs_expected(val, &expected);

        // small uint
        let val = 255u32;
        let expected = vec![EncodingCodes::SmallUint as u8, 255];
        assert_eq_on_serialized_vs_expected(val, &expected);

        // uint
        let val = u32::MAX;
        let mut expected = vec![EncodingCodes::UInt as u8];
        expected.append(&mut vec![255; 4]);
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_u64() {
        // ulong0
        let val = 0u64;
        let expected = vec![EncodingCodes::Ulong0 as u8];
        assert_eq_on_serialized_vs_expected(val, &expected);

        // small ulong
        let val = 255u64;
        let expected = vec![EncodingCodes::SmallUlong as u8, 255];
        assert_eq_on_serialized_vs_expected(val, &expected);

        // ulong
        let val = u64::MAX;
        let mut expected = vec![EncodingCodes::ULong as u8];
        expected.append(&mut vec![255u8; 8]);
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_f32() {
        let val = f32::MIN;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, &expected);

        let val = -123.456f32;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, &expected);

        let val = 0.0f32;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, &expected);

        let val = 123.456f32;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, &expected);

        let val = f32::MAX;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_f64() {
        let val = 123.456f64;
        let mut expected = vec![EncodingCodes::Double as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_char() {
        let val = 'c';
        let mut expected = vec![EncodingCodes::Char as u8];
        expected.append(&mut (val as u32).to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_str() {
        const SMALL_STRING_VALUIE: &str = "Small String";
        const LARGE_STRING_VALUIE: &str = r#"Large String: 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog."#;

        // str8
        let val = SMALL_STRING_VALUIE;
        let len = val.len() as u8;
        let mut expected = vec![EncodingCodes::Str8 as u8, len];
        expected.append(&mut val.as_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, &expected);

        // str32
        let val = LARGE_STRING_VALUIE;
        let len = val.len() as u32;
        let mut expected = vec![EncodingCodes::Str32 as u8];
        expected.append(&mut len.to_be_bytes().to_vec());
        expected.append(&mut val.as_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_bytes() {
        // serialize_bytes only works with serde_bytes wrapper types `Bytes` and `ByteBuf`
        use serde_bytes::ByteBuf;
        const SMALL_BYTES_VALUE: &[u8] = &[133u8; 200];
        const LARGE_BYTES_VALUE: &[u8] = &[199u8; 1000];

        // vbin8
        let val = ByteBuf::from(SMALL_BYTES_VALUE);
        let len = val.len() as u8;
        let mut expected = vec![EncodingCodes::VBin8 as u8, len];
        expected.append(&mut val.to_vec());
        assert_eq_on_serialized_vs_expected(val, &expected);

        // vbin32
        let val = ByteBuf::from(LARGE_BYTES_VALUE);
        let len = val.len() as u32;
        let mut expected = vec![EncodingCodes::VBin32 as u8];
        expected.append(&mut len.to_be_bytes().to_vec());
        expected.append(&mut val.to_vec());
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_none() {
        let val: Option<()> = None;
        let expected = vec![EncodingCodes::Null as u8];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_some() {
        let val = Some(1i32);
        let expected = super::to_vec(&1i32).unwrap();
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_unit() {
        let val = ();
        let expected = vec![EncodingCodes::Null as u8];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_serialize_vec_as_array() {
        let val = Array::from(vec![1, 2, 3, 4]);
        let expected = vec![
            EncodingCodes::Array8 as u8, // array8
            (2 + 4 * 4) as u8,           // length including `count` and element constructor
            4,                           // count
            EncodingCodes::Int as u8,
            0,
            0,
            0,
            1, // first element as i32
            0,
            0,
            0,
            2, // second element
            0,
            0,
            0,
            3, // third
            0,
            0,
            0,
            4, // fourth
        ];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_serialize_slice_as_array() {
        let val = &[1, 2, 3, 4];
        let val = Array::from(val.to_vec());
        let expected = vec![
            EncodingCodes::Array8 as u8, // array8
            (2 + 4 * 4) as u8,           // length including `count` and element constructor
            4,                           // count
            EncodingCodes::Int as u8,
            0,
            0,
            0,
            1, // first element as i32
            0,
            0,
            0,
            2, // second element
            0,
            0,
            0,
            3, // third
            0,
            0,
            0,
            4, // fourth
        ];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_serialzie_list() {
        // List0
        let val: Vec<i32> = vec![];
        let expected = vec![EncodingCodes::List0 as u8];
        assert_eq_on_serialized_vs_expected(val, &expected);

        // List8
        let val = vec![1, 2, 3, 4];
        let expected = vec![
            EncodingCodes::List8 as u8,
            (1 + 2 * 4) as u8, // length including one byte on count
            4,                 // count
            EncodingCodes::SmallInt as u8,
            1,
            EncodingCodes::SmallInt as u8,
            2,
            EncodingCodes::SmallInt as u8,
            3,
            EncodingCodes::SmallInt as u8,
            4,
        ];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_serialzie_slice_as_list() {
        // slice will call `serialize_tuple`
        let val = [1, 2, 3, 4];
        let expected = vec![
            EncodingCodes::List8 as u8,
            (1 + 2 * 4) as u8, // length including one byte on count
            4,                 // count
            EncodingCodes::SmallInt as u8,
            1,
            EncodingCodes::SmallInt as u8,
            2,
            EncodingCodes::SmallInt as u8,
            3,
            EncodingCodes::SmallInt as u8,
            4,
        ];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_serialize_map() {
        use std::collections::BTreeMap;

        // Map8
        let mut val = BTreeMap::new();
        val.insert("a", 1i32);
        val.insert("m", 2);
        val.insert("q", 3);
        val.insert("p", 4);

        // A BTreeMap will be serialized in the ascending order
        let expected = vec![
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

        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_serialize_symbol() {
        use crate::primitives::Symbol;
        let symbol = Symbol::from("amqp");
        let expected = vec![0xa3 as u8, 0x04, 0x61, 0x6d, 0x71, 0x70];
        assert_eq_on_serialized_vs_expected(symbol, &expected);
    }

    #[test]
    fn test_serialize_descriptor_name() {
        // The descriptor name should just be serialized as a symbol
        let descriptor = Descriptor::Name(Symbol::from("amqp"));
        let expected = vec![0x00, 0xa3, 0x04, 0x61, 0x6d, 0x71, 0x70];
        assert_eq_on_serialized_vs_expected(descriptor, &expected);
    }

    #[test]
    fn test_serialize_descriptor_code() {
        let descriptor = Descriptor::Code(0xf2);
        let expected = vec![0x00, 0x53, 0xf2];
        assert_eq_on_serialized_vs_expected(descriptor, &expected);
    }

    use serde::Serialize;

    #[derive(Serialize)]
    struct Foo {
        a_field: i32,
        b: bool,
    }

    #[test]
    fn test_serialize_non_described_struct() {
        let val = Foo {
            a_field: 13,
            b: true,
        };
        let expected = vec![
            EncodingCodes::List8 as u8,
            1 + 2 + 1, // 1 for count, 2 for i32, 1 for true
            2,         // count
            EncodingCodes::SmallInt as u8,
            13,
            EncodingCodes::BooleanTrue as u8,
        ];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_serialize_described_macro() {
        use crate as serde_amqp;
        use crate::macros::SerializeComposite;

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = 0x13, encoding = "list")]
        struct Foo {
            is_fool: bool,
            a: i32,
        }

        let foo = Foo {
            is_fool: true,
            a: 9,
        };
        let expected = vec![0x00, 0x53, 0x13, 0xc0, 0x04, 0x02, 0x41, 0x54, 0x09];
        assert_eq_on_serialized_vs_expected(foo, &expected);
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_serialize_tuple_struct_with_composite_macro() {
        use crate as serde_amqp;
        use crate::macros::SerializeComposite;

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = 0x13, encoding = "list")]
        struct Foo(bool, i32);

        let foo = Foo(true, 9);
        let expected = vec![0x00, 0x53, 0x13, 0xc0, 0x04, 0x02, 0x41, 0x54, 0x09];
        assert_eq_on_serialized_vs_expected(foo, &expected);
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_serialize_unit_struct_with_composite_macro() {
        use crate as serde_amqp;
        use crate::macros::SerializeComposite;

        let expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            1,
            EncodingCodes::List0 as u8,
        ];

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = 0x01, encoding = "list")]
        struct Foo1;

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = 0x01, encoding = "list")]
        struct Foo2();

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = 0x01, encoding = "list")]
        struct Foo3 {}

        let foo1 = Foo1;
        let buf1 = to_vec(&foo1).unwrap();
        assert_eq!(&buf1[..], &expected);

        let foo2 = Foo2();
        let buf2 = to_vec(&foo2).unwrap();
        assert_eq!(&buf2[..], &expected);

        let foo3 = Foo3 {};
        let buf3 = to_vec(&foo3).unwrap();
        assert_eq!(&buf3[..], &expected);
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_serialize_composite_macro_wrapper() {
        use crate as serde_amqp;
        use crate::macros::SerializeComposite;
        use crate::primitives::Symbol;
        use std::collections::BTreeMap;

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = 0x01, encoding = "basic")]
        struct Wrapper(BTreeMap<Symbol, i32>);

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = 0x1, encoding = "basic")]
        struct Wrapper2 {
            map: BTreeMap<Symbol, i32>,
        }

        let mut map = BTreeMap::new();
        map.insert(Symbol::from("a"), 1);
        map.insert(Symbol::from("b"), 2);
        let wrapper = Wrapper(map.clone());
        let expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            1,
            EncodingCodes::Map8 as u8,
            0x0b,
            4,
            EncodingCodes::Sym8 as u8,
            1,
            0x61, // "a"
            EncodingCodes::SmallInt as u8,
            1,
            EncodingCodes::Sym8 as u8,
            1,
            0x62, // "b"
            EncodingCodes::SmallInt as u8,
            2,
        ];
        assert_eq_on_serialized_vs_expected(wrapper, &expected);

        let wrapper2 = Wrapper2 { map };
        assert_eq_on_serialized_vs_expected(wrapper2, &expected);
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_serialize_composite_with_optional_fields() {
        use crate as serde_amqp;
        use crate::macros::SerializeComposite;

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = 0x13, encoding = "list")]
        struct Foo {
            is_fool: Option<bool>,
            a: Option<i32>,
        }

        let foo = Foo {
            is_fool: None,
            a: None,
        };
        let expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x13,
            EncodingCodes::List0 as u8,
        ];
        assert_eq_on_serialized_vs_expected(foo, &expected);

        let foo = Foo {
            is_fool: Some(true),
            a: None,
        };
        let expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            2,
            1,
            EncodingCodes::BooleanTrue as u8,
        ];
        assert_eq_on_serialized_vs_expected(foo, &expected);

        let foo = Foo {
            is_fool: Some(true),
            a: Some(1),
        };
        let expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            4,
            2,
            EncodingCodes::BooleanTrue as u8,
            EncodingCodes::SmallInt as u8,
            1,
        ];
        assert_eq_on_serialized_vs_expected(foo, &expected);

        let foo = Foo {
            is_fool: None,
            a: Some(1),
        };
        let expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            4,
            2,
            EncodingCodes::Null as u8,
            EncodingCodes::SmallInt as u8,
            1,
        ];
        assert_eq_on_serialized_vs_expected(foo, &expected);

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = 0x13, encoding = "list")]
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
        let expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            4,
            2,
            EncodingCodes::Null as u8,
            EncodingCodes::SmallUint as u8,
            0x13,
        ];
        assert_eq_on_serialized_vs_expected(bar, &expected);
    }

    #[cfg(feature = "serde_amqp_derive")]
    #[test]
    fn test_serialize_composite_tuple_with_optional_fields() {
        use crate as serde_amqp;
        use crate::macros::SerializeComposite;

        #[derive(Debug, SerializeComposite)]
        #[amqp_contract(code = 0x13, encoding = "list")]
        struct Foo(Option<bool>, Option<i32>);

        let foo = Foo(None, None);
        let expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x13,
            EncodingCodes::List0 as u8,
        ];
        assert_eq_on_serialized_vs_expected(foo, &expected);

        let foo = Foo(Some(true), None);
        let expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            2,
            1,
            EncodingCodes::BooleanTrue as u8,
        ];
        assert_eq_on_serialized_vs_expected(foo, &expected);

        let foo = Foo(Some(true), Some(1));
        let expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            4,
            2,
            EncodingCodes::BooleanTrue as u8,
            EncodingCodes::SmallInt as u8,
            1,
        ];
        assert_eq_on_serialized_vs_expected(foo, &expected);

        let foo = Foo(None, Some(1));
        let expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x13,
            EncodingCodes::List8 as u8,
            4,
            2,
            EncodingCodes::Null as u8,
            EncodingCodes::SmallInt as u8,
            1,
        ];
        assert_eq_on_serialized_vs_expected(foo, &expected);
    }

    #[allow(dead_code)]
    #[derive(Serialize)]
    enum Enumeration {
        UnitVariant,
        NewTypeVariant(u32),
        TupleVariant(bool, u64, String),
        StructVariant { id: u32, is_true: bool },
    }

    #[test]
    fn test_serialize_unit_variant() {
        let val = Enumeration::UnitVariant;
        let expected = vec![EncodingCodes::Uint0 as u8];
        assert_eq_on_serialized_vs_expected(val, &expected)
    }

    #[test]
    fn test_serialize_newtype_variant() {
        let val = Enumeration::NewTypeVariant(13);
        let expected = vec![
            EncodingCodes::Map8 as u8,
            1 + 2 * 2, // len
            2,         // count
            EncodingCodes::SmallUint as u8,
            1,
            EncodingCodes::SmallUint as u8,
            13,
        ];
        assert_eq_on_serialized_vs_expected(val, &expected)
    }

    #[test]
    fn test_serialize_tuple_variant() {
        let val = Enumeration::TupleVariant(true, 13, String::from("amqp"));
        let expected = vec![
            0xc1, 0x0f, 0x02, 0x52, 0x02, 0xc0, 0x0a, 0x03, 0x41, 0x53, 0x0d, 0xa1, 0x04, 0x61,
            0x6d, 0x71, 0x70,
        ];
        assert_eq_on_serialized_vs_expected(val, &expected)
    }

    #[test]
    fn test_serialize_struct_variant() {
        let val = Enumeration::StructVariant {
            id: 13,
            is_true: true,
        };
        let expected = vec![
            0xc1, 0x09, 0x02, 0x52, 0x03, 0xc0, 0x04, 0x02, 0x52, 0x0d, 0x41,
        ];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_serializing_dec32() {
        let d32 = Dec32::from([0; 4]);
        let expected = vec![EncodingCodes::Decimal32 as u8, 0, 0, 0, 0];
        assert_eq_on_serialized_vs_expected(d32, &expected);
    }

    #[test]
    fn test_serializing_dec64() {
        let d64 = Dec64::from([0; 8]);
        let expected = vec![EncodingCodes::Decimal64 as u8, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq_on_serialized_vs_expected(d64, &expected);
    }

    #[test]
    fn test_serializing_dec128() {
        let d128 = Dec128::from([0; 16]);
        let expected = vec![
            EncodingCodes::Decimal128 as u8,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
        assert_eq_on_serialized_vs_expected(d128, &expected);
    }

    #[test]
    fn test_serialize_timestamp() {
        let val = Timestamp::from(0);
        let expected = vec![EncodingCodes::Timestamp as u8, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[test]
    fn test_serialize_uuid() {
        let val = Uuid::from([0; 16]);
        let expected = vec![
            EncodingCodes::Uuid as u8,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
        assert_eq_on_serialized_vs_expected(val, &expected);
    }

    #[derive(Debug, Serialize)]
    pub struct NewType<T>(T);

    #[derive(Debug, Serialize)]
    pub struct AnotherNewType<T>(T);

    #[test]
    fn test_serialize_vec_of_tuple() {
        let data = vec![(&NewType(NewType(1i32)), &false, "amqp")];
        let buf = to_vec(&data).unwrap();
        println!("{:#x?}", buf);
    }
}
