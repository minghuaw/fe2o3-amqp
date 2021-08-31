use std::{collections::BTreeMap, convert::TryFrom};

use ordered_float::OrderedFloat;
use serde::ser;
use serde_bytes::ByteBuf;

use crate::{
    error::Error,
    types::{
        Array, Dec128, Dec32, Dec64, Symbol, Timestamp, Uuid, ARRAY, DECIMAL128, DECIMAL32,
        DECIMAL64, SYMBOL, TIMESTAMP, UUID,
    },
    util::NewType,
};

use super::Value;

impl ser::Serialize for Value {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(v) => serializer.serialize_bool(*v),
            Value::Ubyte(v) => serializer.serialize_u8(*v),
            Value::Ushort(v) => serializer.serialize_u16(*v),
            Value::Uint(v) => serializer.serialize_u32(*v),
            Value::Ulong(v) => serializer.serialize_u64(*v),
            Value::Byte(v) => serializer.serialize_i8(*v),
            Value::Short(v) => serializer.serialize_i16(*v),
            Value::Int(v) => serializer.serialize_i32(*v),
            Value::Long(v) => serializer.serialize_i64(*v),
            Value::Float(v) => serializer.serialize_f32(v.into_inner()),
            Value::Double(v) => serializer.serialize_f64(v.into_inner()),
            Value::Decimal32(v) => v.serialize(serializer),
            Value::Decimal64(v) => v.serialize(serializer),
            Value::Decimal128(v) => v.serialize(serializer),
            Value::Char(v) => serializer.serialize_char(*v),
            Value::Timestamp(v) => v.serialize(serializer),
            Value::Uuid(v) => v.serialize(serializer),
            Value::Binary(v) => serializer.serialize_bytes(v.as_slice()),
            Value::String(v) => serializer.serialize_str(&v),
            Value::Symbol(v) => v.serialize(serializer),
            Value::List(v) => v.serialize(serializer),
            Value::Map(v) => v.serialize(serializer),
            Value::Array(v) => v.serialize(serializer),
        }
    }
}

pub fn to_value<T>(val: &T) -> Result<Value, Error>
where
    T: ser::Serialize,
{
    let mut ser = Serializer::new();
    ser::Serialize::serialize(val, &mut ser)
}

pub struct Serializer {
    new_type: NewType,
}

impl Serializer {
    pub fn new() -> Self {
        Self {
            new_type: Default::default(),
        }
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a>;
    type SerializeTuple = SeqSerializer<'a>;
    type SerializeTupleStruct = SeqSerializer<'a>;
    type SerializeStruct = SeqSerializer<'a>;
    type SerializeMap = MapSerializer<'a>;
    type SerializeStructVariant = VariantSerializer<'a>;
    type SerializeTupleVariant = VariantSerializer<'a>;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Bool(v))
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Byte(v))
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Short(v))
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(v))
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        match self.new_type {
            NewType::None => Ok(Value::Long(v)),
            NewType::Timestamp => {
                self.new_type = NewType::None;
                Ok(Value::Timestamp(Timestamp::from(v)))
            }
            _ => Err(Error::InvalidValue),
        }
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Ubyte(v))
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Ushort(v))
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Uint(v))
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Ulong(v))
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float(OrderedFloat::from(v)))
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Double(OrderedFloat::from(v)))
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Char(v))
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        match self.new_type {
            NewType::None => Ok(Value::String(String::from(v))),
            NewType::Symbol => {
                self.new_type = NewType::None;
                Ok(Value::Symbol(Symbol::from(v)))
            }
            _ => Err(Error::InvalidValue),
        }
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        match self.new_type {
            NewType::None => Ok(Value::Binary(ByteBuf::from(v.to_vec()))),
            NewType::Dec32 => {
                self.new_type = NewType::None;
                Ok(Value::Decimal32(Dec32::try_from(v)?))
            }
            NewType::Dec64 => {
                self.new_type = NewType::None;
                Ok(Value::Decimal64(Dec64::try_from(v)?))
            }
            NewType::Dec128 => {
                self.new_type = NewType::None;
                Ok(Value::Decimal128(Dec128::try_from(v)?))
            }
            NewType::Uuid => {
                self.new_type = NewType::Uuid;
                Ok(Value::Uuid(Uuid::try_from(v)?))
            }
            NewType::Timestamp => Err(Error::InvalidValue),
            NewType::Array => Err(Error::InvalidValue),
            NewType::Symbol => Err(Error::InvalidValue),
        }
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _ame: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_u32(variant_index)
    }

    #[inline]
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
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

    #[inline]
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        use crate::constants::DESCRIPTOR;
        use crate::value::VALUE;

        if name == DESCRIPTOR || name == VALUE
        // || name == AMQP_ERROR || name == CONNECTION_ERROR || name == SESSION_ERROR || name == LINK_ERROR
        {
            value.serialize(self)
        } else {
            use ser::SerializeSeq;
            // FIXME: how should enum be represented in Value
            let mut state = self.serialize_seq(Some(2))?;
            state.serialize_element(&variant_index)?;
            state.serialize_element(value)?;
            state.end()
        }
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer::new(self, len.unwrap_or(0)))
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SeqSerializer::new(self, len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

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

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            se: self,
            map: BTreeMap::new(),
        })
    }

    #[inline]
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_tuple(len)
    }

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

pub struct SeqSerializer<'a> {
    se: &'a mut Serializer,
    vec: Vec<Value>,
}

impl<'a> SeqSerializer<'a> {
    pub fn new(se: &'a mut Serializer, len: usize) -> Self {
        Self {
            se,
            vec: Vec::with_capacity(len),
        }
    }
}

impl<'a> AsMut<Serializer> for SeqSerializer<'a> {
    fn as_mut(&mut self) -> &mut Serializer {
        self.se
    }
}

impl<'a> ser::SerializeSeq for SeqSerializer<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let val: Value = value.serialize(self.as_mut())?;
        self.vec.push(val);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self.se.new_type {
            NewType::None => Ok(Value::List(self.vec)),
            NewType::Array => Ok(Value::Array(Array::from(self.vec))),
            _ => Err(Error::InvalidValue),
        }
    }
}

impl<'a> ser::SerializeTuple for SeqSerializer<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let val = value.serialize(self.as_mut())?;
        self.vec.push(val);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::List(self.vec))
    }
}

impl<'a> ser::SerializeTupleStruct for SeqSerializer<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        <Self as ser::SerializeTuple>::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as ser::SerializeTuple>::end(self)
    }
}

// pub struct StructSerializer { }

impl<'a> ser::SerializeStruct for SeqSerializer<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        <Self as ser::SerializeTuple>::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as ser::SerializeTuple>::end(self)
    }
}

pub struct MapSerializer<'a> {
    se: &'a mut Serializer,
    map: BTreeMap<Value, Value>,
}

impl<'a> AsMut<Serializer> for MapSerializer<'a> {
    fn as_mut(&mut self) -> &mut Serializer {
        self.se
    }
}

impl<'a> ser::SerializeMap for MapSerializer<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: serde::Serialize,
        V: serde::Serialize,
    {
        let key = key.serialize(self.as_mut())?;
        let value = value.serialize(self.as_mut())?;
        self.map.insert(key, value);
        Ok(())
    }

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        unreachable!()
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        unreachable!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.map))
    }
}

pub struct VariantSerializer<'a> {
    se: &'a mut Serializer,
    _name: &'static str,
    variant_index: u32,
    _variant: &'static str,
    _num: usize,
    buf: Vec<Value>,
}

impl<'a> VariantSerializer<'a> {
    pub fn new(
        se: &'a mut Serializer,
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
            _num: num,
            buf: Vec::new(),
        }
    }
}

impl<'a> AsMut<Serializer> for VariantSerializer<'a> {
    fn as_mut(&mut self) -> &mut Serializer {
        self.se
    }
}

impl<'a> ser::SerializeTupleVariant for VariantSerializer<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value = value.serialize(self.as_mut())?;
        self.buf.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let value = Value::List(self.buf);
        let index = Value::Uint(self.variant_index);
        Ok(Value::List(vec![index, value]))
    }
}

impl<'a> ser::SerializeStructVariant for VariantSerializer<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        <Self as ser::SerializeTupleVariant>::serialize_field(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as ser::SerializeTupleVariant>::end(self)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Serialize;

    use crate::{
        types::{Array, Timestamp},
        value::Value,
    };

    use super::to_value;

    fn assert_eq_on_value_vs_expected<T: Serialize>(val: T, expected: Value) {
        let value: Value = to_value(&val).unwrap();
        assert_eq!(value, expected)
    }

    #[test]
    fn test_serialize_value_bool() {
        let val = true;
        let expected: Value = Value::Bool(true);
        assert_eq_on_value_vs_expected(val, expected)
    }

    #[test]
    fn test_serialize_value_timestamp() {
        let val = Timestamp::from(131313);
        let expected = Value::Timestamp(Timestamp::from(131313));
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_list() {
        let val = vec![1i32, 2, 3, 4];
        let expected = Value::List(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_array() {
        let val = Array::from(vec![1i32, 2, 3, 4]);
        let expected = Value::Array(Array::from(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]));
        println!("{:?}", val);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_map() {
        let mut val = BTreeMap::new();
        val.insert("a", 1i32);
        val.insert("m", 2);
        val.insert("q", 3);
        val.insert("p", 4);
        let expected = Value::Map(
            val.iter()
                .map(|(k, v)| (to_value(k).unwrap(), to_value(v).unwrap()))
                .collect(),
        );
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_unit_variant() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A,
            B,
            C,
        }

        let val = Foo::B;
        let expected = Value::Uint(1);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_newtype_variant() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A(String),
            B(u64),
        }

        let val = Foo::B(13);
        let expected = Value::List(vec![Value::Uint(1), Value::Ulong(13)]);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_tuple_variant() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A(u32, bool),
            B(i32, String),
        }
        let val = Foo::B(13, "amqp".to_string());
        let expected = Value::List(vec![
            Value::Uint(1),
            Value::List(vec![Value::Int(13), Value::String(String::from("amqp"))]),
        ]);
        assert_eq_on_value_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_value_struct_variant() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum Foo {
            A { num: u32, is_a: bool },
            B { signed_num: i32, amqp: String },
        }

        let val = Foo::A {
            num: 13,
            is_a: true,
        };
        let expected = Value::List(vec![
            Value::Uint(0),
            Value::List(vec![Value::Uint(13), Value::Bool(true)]),
        ]);
        assert_eq_on_value_vs_expected(val, expected);
    }
}
