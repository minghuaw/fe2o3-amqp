use std::io::Write;

use serde::{
    ser::{self, SerializeMap, SerializeSeq},
    Serialize,
};

use crate::{
    constructor::EncodingCodes,
    described::{DESCRIBED_BASIC, DESCRIBED_LIST, DESCRIBED_MAP},
    descriptor::DESCRIPTOR,
    error::Error,
    types::SYMBOL,
    value::U32_MAX_AS_USIZE,
};

pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, Error>
where
    T: Serialize,
{
    let mut writer = Vec::new(); // TODO: pre-allocate capacity
    let mut serializer = Serializer::new(&mut writer);
    value.serialize(&mut serializer)?;
    Ok(writer)
}

enum NewType {
    None,
    Symbol,
}

impl Default for NewType {
    fn default() -> Self {
        Self::None
    }
}

#[repr(u8)]
enum StructEncoding {
    None,
    DescribedList,
    DescribedMap,
    DescribedBasic,
}

impl Default for StructEncoding {
    fn default() -> Self {
        Self::None
    }
}

pub struct Serializer<W> {
    writer: W,
    newtype: NewType,
    encoding: StructEncoding,
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            newtype: Default::default(),
            encoding: Default::default(),
        }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }

    pub fn symbol(writer: W) -> Self {
        Self {
            writer,
            newtype: NewType::Symbol,
            encoding: Default::default(),
        }
    }

    pub fn described_list(writer: W) -> Self {
        Self {
            writer,
            newtype: Default::default(),
            encoding: StructEncoding::DescribedList,
        }
    }

    pub fn described_map(writer: W) -> Self {
        Self {
            writer,
            newtype: Default::default(),
            encoding: StructEncoding::DescribedMap,
        }
    }

    pub fn described_basic(writer: W) -> Self {
        Self {
            writer,
            newtype: Default::default(),
            encoding: StructEncoding::DescribedBasic,
        }
    }
}

impl<'a, W: Write + 'a> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Compound<'a, W>;
    type SerializeTuple = Compound<'a, W>;
    type SerializeMap = Compound<'a, W>;
    type SerializeTupleStruct = Compound<'a, W>;
    type SerializeStruct = DescribedCompound<'a, W>;
    type SerializeTupleVariant = VariantSerializer<'a, W>;

    // The variant struct will be serialized as Value in DescribedCompound
    type SerializeStructVariant = VariantSerializer<'a, W>;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        match v {
            true => {
                let buf = [EncodingCodes::BooleanTrue as u8];
                self.writer.write_all(&buf)
            }
            false => {
                let buf = [EncodingCodes::BooleanFalse as u8];
                self.writer.write_all(&buf)
            }
        }
        .map_err(Into::into)
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        let buf = [EncodingCodes::Byte as u8, v as u8];
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        let code = [EncodingCodes::Short as u8];
        self.writer.write_all(&code)?;
        let buf = v.to_be_bytes();
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        match v {
            val @ -128..=127 => {
                let buf = [EncodingCodes::SmallInt as u8, val as u8];
                self.writer.write_all(&buf)
            }
            val @ _ => {
                let code = [EncodingCodes::Int as u8];
                self.writer.write_all(&code)?;
                let buf: [u8; 4] = val.to_be_bytes();
                self.writer.write_all(&buf)
            }
        }
        .map_err(Into::into)
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        match v {
            val @ -128..=127 => {
                let buf = [EncodingCodes::SmallLong as u8, val as u8];
                self.writer.write_all(&buf)
            }
            val @ _ => {
                let code = [EncodingCodes::Long as u8];
                self.writer.write_all(&code)?;
                let buf: [u8; 8] = val.to_be_bytes();
                self.writer.write_all(&buf)
            }
        }
        .map_err(Into::into)
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        let buf = [EncodingCodes::Ubyte as u8, v];
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        let code = [EncodingCodes::Ushort as u8];
        let buf: [u8; 2] = v.to_be_bytes();
        self.writer.write_all(&code)?;
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        match v {
            // uint0
            0 => {
                let buf = [EncodingCodes::Uint0 as u8];
                self.writer.write_all(&buf)
            }
            // smalluint
            val @ 1..=255 => {
                let buf = [EncodingCodes::SmallUint as u8, val as u8];
                self.writer.write_all(&buf)
            }
            // uint
            val @ _ => {
                let code = [EncodingCodes::Uint as u8];
                let buf: [u8; 4] = val.to_be_bytes();
                self.writer.write_all(&code)?;
                self.writer.write_all(&buf)
            }
        }
        .map_err(Into::into)
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        match v {
            // ulong0
            0 => {
                let buf = [EncodingCodes::Ulong0 as u8];
                self.writer.write_all(&buf)
            }
            // small ulong
            val @ 1..=255 => {
                let buf = [EncodingCodes::SmallUlong as u8, val as u8];
                self.writer.write_all(&buf)
            }
            // ulong
            val @ _ => {
                let code = [EncodingCodes::Ulong as u8];
                let buf: [u8; 8] = val.to_be_bytes();
                self.writer.write_all(&code)?;
                self.writer.write_all(&buf)
            }
        }
        .map_err(Into::into)
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        let code = [EncodingCodes::Float as u8];
        let buf = v.to_be_bytes();
        self.writer.write_all(&code)?;
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        let code = [EncodingCodes::Double as u8];
        let buf = v.to_be_bytes();
        self.writer.write_all(&code)?;
        self.writer.write_all(&buf).map_err(Into::into)
    }

    // `char` in rust is a subset of the unicode code points and
    // can be directly treated as u32
    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let code = [EncodingCodes::Char as u8];
        let buf = (v as u32).to_be_bytes();
        self.writer.write_all(&code)?;
        self.writer.write_all(&buf).map_err(Into::into)
    }

    // String slices are always valid utf-8
    // `String` isd utf-8 encoded
    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let l = v.len();
        match self.newtype {
            NewType::Symbol => {
                match l {
                    // sym8
                    0..=255 => {
                        let code = [EncodingCodes::Sym8 as u8, l as u8];
                        self.writer.write_all(&code)?;
                    }
                    256..=U32_MAX_AS_USIZE => {
                        let code = [EncodingCodes::Sym32 as u8];
                        let width = (l as u32).to_be_bytes();
                        self.writer.write_all(&code)?;
                        self.writer.write_all(&width)?;
                    }
                    _ => return Err(Error::Message("Too long".into())),
                }
                self.newtype = NewType::None;
            }
            _ => {
                match l {
                    // str8-utf8
                    0..=255 => {
                        let code = [EncodingCodes::Str8 as u8, l as u8];
                        // let width: [u8; 1] = (l as u8).to_be_bytes();
                        self.writer.write_all(&code)?;
                        // self.writer.write_all(&width)?;
                    }
                    // str32-utf8
                    256..=U32_MAX_AS_USIZE => {
                        let code = [EncodingCodes::Str32 as u8];
                        let width: [u8; 4] = (l as u32).to_be_bytes();
                        self.writer.write_all(&code)?;
                        self.writer.write_all(&width)?;
                    }
                    _ => return Err(Error::Message("Too long".into())),
                }
            }
        }

        self.writer.write_all(v.as_bytes()).map_err(Into::into)
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let l = v.len();
        match l {
            // vbin8
            0..=255 => {
                let code = [EncodingCodes::VBin8 as u8];
                let width: [u8; 1] = (l as u8).to_be_bytes();
                self.writer.write_all(&code)?;
                self.writer.write_all(&width)?;
            }
            // vbin32
            256..=U32_MAX_AS_USIZE => {
                let code = [EncodingCodes::VBin32 as u8];
                let width: [u8; 4] = (l as u32).to_be_bytes();
                self.writer.write_all(&code)?;
                self.writer.write_all(&width)?;
            }
            _ => return Err(Error::Message("Too long".into())),
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

    // Treat newtype structs as insignificant wrappers around the data they contain
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
            self.newtype = NewType::Symbol;
        }
        value.serialize(self)
    }

    // Treat newtype variant as insignificant wrappers around the data they contain
    #[inline]
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        let mut map_se = self.serialize_map(Some(1))?;
        map_se.serialize_entry(&variant_index, value)?;
        SerializeMap::end(map_se)
    }

    // A variably sized heterogeneous sequence of values
    //
    // This will be encoded as primitive type `List`
    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        match len {
            Some(num) => Ok(Compound::new(self, num)),
            None => Err(Error::Message("Length must be known".into())),
        }
    }

    // A statically sized heterogeneous sequence of values
    // for which the length will be known at deserialization
    // time without looking at the serialized data
    //
    // This will be encoded as primitive type `List`
    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    // A named tuple, for example `struct Rgb(u8, u8, u8)`
    //
    // The tuple struct looks rather like a rust-exclusive data type.
    // Thus this will be treated the same as a tuple (as in JSON and AVRO)
    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        // This will never be called on a Described type because the Decribed type is
        // a struct.
        // Simply serialize the content as seq
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        match len {
            Some(num) => Ok(Compound::new(self, num * 2)),
            None => Err(Error::Message("Length must be known".into())),
        }
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
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        if name == DESCRIPTOR {
            return Ok(DescribedCompound::descriptor(
                StructRole::Descriptor,
                self,
                len,
            ));
        }

        match self.encoding {
            // A None state indicates a freshly instantiated serializer
            StructEncoding::None => {
                if name == DESCRIBED_LIST {
                    self.encoding = StructEncoding::DescribedList;
                    let code = [EncodingCodes::DescribedType as u8];
                    self.writer.write_all(&code)?;
                    Ok(DescribedCompound::list_value(
                        StructRole::Described,
                        self,
                        len,
                    ))
                } else if name == DESCRIBED_MAP {
                    self.encoding = StructEncoding::DescribedMap;
                    let code = [EncodingCodes::DescribedType as u8];
                    self.writer.write_all(&code)?;
                    Ok(DescribedCompound::map_value(
                        StructRole::Described,
                        self,
                        len,
                    ))
                } else if name == DESCRIBED_BASIC {
                    self.encoding = StructEncoding::DescribedBasic;
                    let code = [EncodingCodes::DescribedType as u8];
                    self.writer.write_all(&code)?;
                    Ok(DescribedCompound::basic_value(
                        StructRole::Described,
                        self,
                        len,
                    ))
                } else if name == DESCRIPTOR {
                    Ok(DescribedCompound::descriptor(
                        StructRole::Descriptor,
                        self,
                        len,
                    ))
                } else {
                    // Only non-described struct will go to this branch
                    Ok(DescribedCompound::list_value(StructRole::Value, self, len))
                }
            }
            StructEncoding::DescribedBasic => {
                Ok(DescribedCompound::basic_value(StructRole::Value, self, len))
            }
            StructEncoding::DescribedList => {
                Ok(DescribedCompound::list_value(StructRole::Value, self, len))
            }
            StructEncoding::DescribedMap => {
                Ok(DescribedCompound::map_value(StructRole::Value, self, len))
            }
        }
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

/// Serializer for Compound types. Compound types include:
/// 1. List
/// 2. Map
pub struct Compound<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    num: usize,   // number of element
    buf: Vec<u8>, // byte buffer
}

impl<'a, W: 'a> Compound<'a, W> {
    fn new(se: &'a mut Serializer<W>, num: usize) -> Self {
        Self {
            se,
            num,
            buf: Vec::new(),
        }
    }
}

// This requires some hacking way of getting the constructor (EncodingCode)
// for the type. Use TypeId?
//
// Serialize into a List
// List requires knowing the total number of bytes after serialized
impl<'a, W: Write + 'a> ser::SerializeSeq for Compound<'a, W> {
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
        write_seq(&mut se.writer, num, buf)
    }
}

fn write_seq<'a, W: Write + 'a>(writer: &'a mut W, num: usize, buf: Vec<u8>) -> Result<(), Error> {
    let len = buf.len();

    // if `len` < 255, `num` must be smaller than 255
    match len {
        0 => {
            let code = [EncodingCodes::List0 as u8];
            writer.write_all(&code)?;
        }
        1..=255 => {
            let code = [EncodingCodes::List8 as u8, len as u8, num as u8];
            writer.write_all(&code)?;
            writer.write_all(&buf)?;
        }
        256..=U32_MAX_AS_USIZE => {
            let code = [EncodingCodes::List32 as u8];
            let len: [u8; 4] = (len as u32).to_be_bytes();
            let num: [u8; 4] = (num as u32).to_be_bytes();
            writer.write_all(&code)?;
            writer.write_all(&len)?;
            writer.write_all(&num)?;
            writer.write_all(&buf)?;
        }
        _ => return Err(Error::Message("Too long".into())),
    }
    Ok(())
}

// Serialize into a List
// List requires knowing the total number of bytes after serialized
impl<'a, W: Write + 'a> ser::SerializeTuple for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        <Self as SerializeSeq>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as SerializeSeq>::end(self)
    }
}

// Map is a compound type and thus requires knowing the size of the total number
// of bytes
impl<'a, W: Write + 'a> ser::SerializeMap for Compound<'a, W> {
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
        Ok(())
    }

    #[inline]
    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let mut serializer = Serializer::new(&mut self.buf);
        key.serialize(&mut serializer)
    }

    #[inline]
    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let mut serializer = Serializer::new(&mut self.buf);
        value.serialize(&mut serializer)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        let Self { se, num, buf } = self;
        write_map(&mut se.writer, num, buf)
    }
}

fn write_map<'a, W: Write + 'a>(writer: &'a mut W, num: usize, buf: Vec<u8>) -> Result<(), Error> {
    let len = buf.len();

    match len {
        0..=255 => {
            let code = [EncodingCodes::Map8 as u8, len as u8, num as u8];
            writer.write_all(&code)?;
            writer.write_all(&buf)?;
        }
        256..=U32_MAX_AS_USIZE => {
            let code = [EncodingCodes::Map32 as u8];
            let len = (len as u32).to_be_bytes();
            let num = (num as u32).to_be_bytes();
            writer.write_all(&code)?;
            writer.write_all(&len)?;
            writer.write_all(&num)?;
            writer.write_all(&buf)?;
        }
        _ => return Err(Error::Message("Too long".into())),
    }
    Ok(())
}

// Serialize into a List with
impl<'a, W: Write + 'a> ser::SerializeTupleStruct for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        <Self as SerializeSeq>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as SerializeSeq>::end(self)
    }
}

pub enum StructRole {
    Described,
    Descriptor,
    Value,
}

pub enum ValueType {
    Basic,
    List,
    Map,
}

pub struct DescribedCompound<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    role: StructRole,
    val_ty: ValueType,
    num: usize,
    buf: Vec<u8>,
}

#[inline]
pub fn init_vec(role: &StructRole) -> Vec<u8> {
    match role {
        &StructRole::Value => Vec::new(),
        _ => Vec::with_capacity(0),
    }
}

impl<'a, W: 'a> DescribedCompound<'a, W> {
    pub fn descriptor(role: StructRole, se: &'a mut Serializer<W>, num: usize) -> Self {
        let buf = init_vec(&role);
        Self {
            se,
            role,
            val_ty: ValueType::Basic,
            num,
            buf,
        }
    }

    pub fn basic_value(role: StructRole, se: &'a mut Serializer<W>, num: usize) -> Self {
        let buf = init_vec(&role);
        Self {
            se,
            role,
            val_ty: ValueType::Basic,
            num,
            buf,
        }
    }

    pub fn list_value(role: StructRole, se: &'a mut Serializer<W>, num: usize) -> Self {
        let buf = init_vec(&role);
        Self {
            se,
            role,
            val_ty: ValueType::List,
            num,
            buf,
        }
    }

    pub fn map_value(role: StructRole, se: &'a mut Serializer<W>, num: usize) -> Self {
        let buf = init_vec(&role);
        Self {
            se,
            role,
            val_ty: ValueType::Map,
            num,
            buf,
        }
    }
}

impl<'a, W: 'a> AsMut<Serializer<W>> for DescribedCompound<'a, W> {
    fn as_mut(&mut self) -> &mut Serializer<W> {
        self.se
    }
}

// Serialize into a list
impl<'a, W: Write + 'a> ser::SerializeStruct for DescribedCompound<'a, W> {
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
        match self.role {
            StructRole::Described => value.serialize(self.as_mut()),
            StructRole::Descriptor => value.serialize(self.as_mut()),
            StructRole::Value => match self.val_ty {
                ValueType::Basic => value.serialize(self.as_mut()),
                ValueType::List => {
                    let mut serializer = Serializer::described_list(&mut self.buf);
                    value.serialize(&mut serializer)
                }
                ValueType::Map => {
                    let mut serializer = Serializer::described_map(&mut self.buf);
                    key.serialize(&mut serializer)?;
                    value.serialize(&mut serializer)
                }
            },
        }
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self.role {
            StructRole::Described => Ok(()),
            StructRole::Descriptor => Ok(()),
            StructRole::Value => match self.val_ty {
                ValueType::Basic => Ok(()),
                ValueType::List => write_seq(&mut self.se.writer, self.num, self.buf),
                ValueType::Map => write_map(&mut self.se.writer, self.num, self.buf),
            },
        }
    }
}

pub struct VariantSerializer<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    _name: &'static str,
    variant_index: u32,
    _variant: &'static str,
    num: usize,
    buf: Vec<u8>,
}

impl<'a, W: 'a> VariantSerializer<'a, W> {
    pub fn new(
        se: &'a mut Serializer<W>,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        num: usize, // number of field in the tuple
    ) -> Self {
        Self {
            se,
            _name: name,
            variant_index,
            _variant: variant,
            num,
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
        // let seq_se = Compound::new(&mut se, self.num);
        // seq_se.serialize_element(value)
        value.serialize(&mut se)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut value = Vec::new();
        write_seq(&mut value, self.num, self.buf)?;

        let mut kv = Vec::new();
        let mut se = Serializer::new(&mut kv);
        ser::Serialize::serialize(&self.variant_index, &mut se)?;
        kv.append(&mut value);
        write_map(&mut self.se.writer, 1, kv)
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
    use crate::{constructor::EncodingCodes, described::Described, descriptor::Descriptor};

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
        let val = 0i8;
        let expected = vec![EncodingCodes::Byte as u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = i8::MIN;
        let expected = vec![EncodingCodes::Byte as u8, 128u8];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = i8::MAX;
        let expected = vec![EncodingCodes::Byte as u8, 127u8];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_i16() {
        let val = 0i16;
        let expected = vec![EncodingCodes::Short as u8, 0, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = -1i16;
        let expected = vec![EncodingCodes::Short as u8, 255, 255];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_i32() {
        // small int
        let val = 0i32;
        let expected = vec![EncodingCodes::SmallInt as u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        // int
        let val = i32::MAX;
        let expected = vec![EncodingCodes::Int as u8, 127, 255, 255, 255];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_i64() {
        // small long
        let val = 0i64;
        let expected = vec![EncodingCodes::SmallLong as u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

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
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_u8() {
        let val = u8::MIN;
        let expected = vec![EncodingCodes::Ubyte as u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = u8::MAX;
        let expected = vec![EncodingCodes::Ubyte as u8, 255];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_u16() {
        let val = 0u16;
        let expected = vec![EncodingCodes::Ushort as u8, 0, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = 131u16;
        let expected = vec![EncodingCodes::Ushort as u8, 0, 131];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = 65535u16;
        let expected = vec![EncodingCodes::Ushort as u8, 255, 255];
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
        expected.append(&mut vec![255; 4]);
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
        expected.append(&mut vec![255u8; 8]);
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_f32() {
        let val = f32::MIN;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = -123.456f32;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = 0.0f32;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = 123.456f32;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = f32::MAX;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_f64() {
        let val = 123.456f64;
        let mut expected = vec![EncodingCodes::Double as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_char() {
        let val = 'c';
        let mut expected = vec![EncodingCodes::Char as u8];
        expected.append(&mut (val as u32).to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);
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
        let output = to_vec(&val).unwrap();
        println!("{:?}", output);
        let mut expected = vec![EncodingCodes::Str8 as u8, len];
        expected.append(&mut val.as_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);

        // str32
        let val = LARGE_STRING_VALUIE;
        let len = val.len() as u32;
        let mut expected = vec![EncodingCodes::Str32 as u8];
        expected.append(&mut len.to_be_bytes().to_vec());
        expected.append(&mut val.as_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);
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
        assert_eq_on_serialized_vs_expected(val, expected);

        // vbin32
        let val = ByteBuf::from(LARGE_BYTES_VALUE);
        let len = val.len() as u32;
        let mut expected = vec![EncodingCodes::VBin32 as u8];
        expected.append(&mut len.to_be_bytes().to_vec());
        expected.append(&mut val.to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_none() {
        let val: Option<()> = None;
        let expected = vec![EncodingCodes::Null as u8];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_some() {
        let val = Some(1i32);
        let expected = super::to_vec(&1i32).unwrap();
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_unit() {
        let val = ();
        let expected = vec![EncodingCodes::Null as u8];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_symbol() {
        use crate::types::Symbol;
        let symbol = Symbol::new("amqp".into());
        let expected = vec![0xa3 as u8, 0x04, 0x61, 0x6d, 0x71, 0x70];
        assert_eq_on_serialized_vs_expected(symbol, expected);
    }

    #[test]
    fn test_serialize_descriptor_name() {
        // The descriptor name should just be serialized as a symbol
        let descriptor = Descriptor::new(Some(String::from("amqp")), None);
        let expected = vec![0xa3 as u8, 0x04, 0x61, 0x6d, 0x71, 0x70];
        assert_eq_on_serialized_vs_expected(descriptor, expected);
    }

    #[test]
    fn test_serialize_descriptor_code() {
        let descriptor = Descriptor::new(Some(String::from("amqp")), Some(0xf2));
        let expected = vec![0x53, 0xf2];
        assert_eq_on_serialized_vs_expected(descriptor, expected);
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
        let output = to_vec(&val).unwrap();
        println!("{:?}", &output);
    }

    #[test]
    fn test_serialize_described_basic_type() {
        let value = String::from("amqp");
        let descriptor = Descriptor::new(Some("val".to_string()), Some(100));
        let described = Described::new(crate::described::EncodingType::Basic, descriptor, &value);
        let mut expected = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            100u64 as u8,
            EncodingCodes::Str8 as u8,
            4 as u8,
        ];
        expected.append(&mut value.as_bytes().into());
        assert_eq_on_serialized_vs_expected(described, expected);
    }

    #[test]
    fn test_serialize_described_list_type() {
        let value = Foo {
            a_field: 13,
            b: true,
        };
        let descriptor = Descriptor::new(Some("Foo".to_string()), Some(13));
        let described = Described::new(crate::described::EncodingType::List, descriptor, &value);
        let expected = vec![
            EncodingCodes::DescribedType as u8, // Described type contructor
            EncodingCodes::SmallUlong as u8,    // Descriptor code
            13u8,
            EncodingCodes::List8 as u8,    // List
            3u8,                           // List length in bytes
            2u8,                           // Number of items in list
            EncodingCodes::SmallInt as u8, // Constructor of first item
            13u8,
            EncodingCodes::BooleanTrue as u8, // Constructor of second item
        ];
        assert_eq_on_serialized_vs_expected(described, expected);
    }

    #[test]
    fn test_serialize_described_map_type() {
        let value = Foo {
            a_field: 13,
            b: true,
        };
        let descriptor = Descriptor::new(Some("Foo".to_string()), Some(13));
        let described = Described::new(crate::described::EncodingType::Map, descriptor, &value);
        let output = to_vec(&described).unwrap();
        println!("{:?}", output);
    }

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
        let output = to_vec(&val).unwrap();
        println!("{:?}", output);

        unimplemented!()
    }

    #[test]
    fn test_serialize_newtype_variant() {
        let val = Enumeration::NewTypeVariant(13);
        let output = to_vec(&val).unwrap();
        println!("{:?}", output);

        unimplemented!()
    }

    #[test]
    fn test_serialize_tuple_variant() {
        let val = Enumeration::TupleVariant(true, 13, String::from("amqp"));
        let output = to_vec(&val).unwrap();
        println!("{:?}", output);

        unimplemented!()
    }

    #[test]
    fn test_serialize_struct_variant() {
        let val = Enumeration::StructVariant {
            id: 13,
            is_true: true,
        };
        let output = to_vec(&val).unwrap();
        println!("{:?}", output);

        unimplemented!()
    }

    #[test]
    fn test_serialize_described_unit_variant() {
        unimplemented!()
    }

    #[test]
    fn test_serialize_described_newtype_variant() {
        unimplemented!()
    }

    #[test]
    fn test_serialize_described_tuple_variant() {
        unimplemented!()
    }

    #[test]
    fn test_serialize_described_struct_variant() {
        unimplemented!()
    }
}
