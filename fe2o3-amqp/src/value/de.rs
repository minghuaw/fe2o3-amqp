use std::{convert::TryInto};

use ordered_float::OrderedFloat;
use serde::de::{self, VariantAccess};

use crate::{error::Error, format_code::EncodingCodes};

use super::{Value, VALUE};

enum Field {
    Null,
    Bool,
    Ubyte,
    Ushort,
    Uint,
    Ulong,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    Decimal32,
    Decimal64,
    Decimal128,
    Char,
    Timestamp,
    Uuid,
    Binary,
    String,
    Symbol,
    List,
    Map,
    Array
}

struct FieldVisitor { }

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("field of enum Value")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error, 
    {
        println!(">>> Debug visit_u8 {:x?}", v);

        let field = match v.try_into()
            .map_err(|err: Error| de::Error::custom(err.to_string()))? 
        {
            EncodingCodes::Null => Field::Null,
            EncodingCodes::Boolean | EncodingCodes::BooleanFalse | EncodingCodes::BooleanTrue => Field::Bool,
            EncodingCodes::Ubyte => Field::Ubyte,
            EncodingCodes::Ushort => Field::Ushort,
            EncodingCodes::Uint | EncodingCodes::Uint0 | EncodingCodes::SmallUint => Field::Uint,
            EncodingCodes::Ulong | EncodingCodes::Ulong0 | EncodingCodes::SmallUlong => Field::Ulong,
            EncodingCodes::Byte => Field::Byte,
            EncodingCodes::Short => Field::Short,
            EncodingCodes::Int | EncodingCodes::SmallInt => Field::Int,
            EncodingCodes::Long | EncodingCodes::SmallLong => Field::Long,
            EncodingCodes::Float => Field::Float,
            EncodingCodes::Double => Field::Double,
            EncodingCodes::Decimal32 => Field::Decimal32,
            EncodingCodes::Decimal64 => Field::Decimal64,
            EncodingCodes::Decimal128 => Field::Decimal128,
            EncodingCodes::Char => Field::Char,
            EncodingCodes::Timestamp => Field::Timestamp,
            EncodingCodes::Uuid => Field::Uuid,
            EncodingCodes::VBin32 | EncodingCodes::VBin8 => Field::Binary,
            EncodingCodes::Str32 | EncodingCodes::Str8 => Field::String,
            EncodingCodes::Sym32 | EncodingCodes::Sym8 => Field::Symbol,
            EncodingCodes::List0 | EncodingCodes::List32 | EncodingCodes::List8 => Field::List,
            EncodingCodes::Map32 | EncodingCodes::Map8 => Field::Map,
            EncodingCodes::Array32 | EncodingCodes::Array8 => Field::Array,
            
            // The `Value` type cannot hold a `Described` type
            EncodingCodes::DescribedType => return Err(de::Error::custom("Described type in Value enum is not supported yet"))
            // EncodingCodes::DescribedType => Field::List, // could probably treat it as a list of two items
        };
        Ok(field)
    }
}

impl<'de> de::Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> 
    {
        deserializer.deserialize_identifier(FieldVisitor { })    
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum Value")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>, 
    {
        let (val, de) = data.variant()?;

        match val {
            Field::Null => {
                let _: () = de.newtype_variant()?;
                Ok(Value::Null)
            },
            Field::Bool => {
                let val = de.newtype_variant()?;
                Ok(Value::Bool(val))
            },
            Field::Ubyte => {
                let val = de.newtype_variant()?;
                Ok(Value::Ubyte(val))
            },
            Field::Ushort => {
                let val = de.newtype_variant()?;
                Ok(Value::Ushort(val))
            },
            Field::Uint => {
                let val = de.newtype_variant()?;
                Ok(Value::Uint(val))
            },
            Field::Ulong => {
                let val = de.newtype_variant()?;
                Ok(Value::Ulong(val))
            },
            Field::Byte => {
                let val = de.newtype_variant()?;
                Ok(Value::Byte(val))
            },
            Field::Short => {
                let val = de.newtype_variant()?;
                Ok(Value::Short(val))
            },
            Field::Int => {
                let val = de.newtype_variant()?;
                Ok(Value::Int(val))
            },
            Field::Long => {
                let val = de.newtype_variant()?;
                Ok(Value::Long(val))
            },
            Field::Float => {
                let val: f32 = de.newtype_variant()?;
                Ok(Value::Float(OrderedFloat::from(val)))
            },
            Field::Double => {
                let val: f64 = de.newtype_variant()?;
                Ok(Value::Double(OrderedFloat::from(val)))
            },
            Field::Decimal32 => {
                let val = de.newtype_variant()?;
                Ok(Value::Decimal32(val))
            },
            Field::Decimal64 => {
                let val = de.newtype_variant()?;
                Ok(Value::Decimal64(val))
            },
            Field::Decimal128 => {
                let val = de.newtype_variant()?;
                Ok(Value::Decimal128(val))
            },
            Field::Char => {
                let val = de.newtype_variant()?;
                Ok(Value::Char(val))
            },
            Field::Timestamp => {
                let val = de.newtype_variant()?;
                Ok(Value::Timestamp(val))
            },
            Field::Uuid => {
                let val = de.newtype_variant()?;
                Ok(Value::Uuid(val))
            },
            Field::Binary => {
                let val = de.newtype_variant()?;
                Ok(Value::Binary(val))
            },
            Field::String => {
                let val = de.newtype_variant()?;
                Ok(Value::String(val))
            },
            Field::Symbol => {
                let val = de.newtype_variant()?;
                Ok(Value::Symbol(val))
            },
            Field::List => {
                let val = de.newtype_variant()?;
                Ok(Value::List(val))
            },
            Field::Map => {
                let val = de.newtype_variant()?;
                Ok(Value::Map(val))
            },
            Field::Array => {
                let val = de.newtype_variant()?;
                Ok(Value::Array(val))
            }
        }
    }

    // #[inline]
    // fn visit_unit<E>(self) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Null)
    // }

    // #[inline]
    // fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Bool(v))
    // }

    // #[inline]
    // fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Ubyte(v))
    // }

    // #[inline]
    // fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Ushort(v))
    // }

    // #[inline]
    // fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Uint(v))
    // }

    // #[inline]
    // fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Ulong(v))
    // }

    // #[inline]
    // fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Byte(v))
    // }

    // #[inline]
    // fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Short(v))
    // }

    // #[inline]
    // fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Int(v))
    // }

    // #[inline]
    // fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Long(v))
    // }

    // #[inline]
    // fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Float(OrderedFloat::from(v)))
    // }

    // #[inline]
    // fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Double(OrderedFloat::from(v)))
    // }

    // #[inline]
    // fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::String(String::from(v)))
    // }

    // #[inline]
    // fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::String(v))
    // }

    // #[inline]
    // fn visit_none<E>(self) -> Result<Self::Value, E>
    // where
    //     E: de::Error,
    // {
    //     Ok(Value::Null)
    // }

    // #[inline]
    // fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    // where
    //     A: de::SeqAccess<'de>,
    // {
    //     let mut vec = Vec::new();
    //     let mut first_elem_index = None;
    //     let mut contains_same_variant = true;

    //     while let Some(elem) = seq.next_element::<Value>()? {
    //         match first_elem_index {
    //             Some(n) => {
    //                 let curr_elem_index = elem.index();
    //                 if n != curr_elem_index {
    //                     contains_same_variant = false;
    //                 }
    //             }
    //             None => first_elem_index = Some(elem.index()),
    //         }

    //         vec.push(elem)
    //     }

    //     match contains_same_variant {
    //         true => Ok(Value::Array(Array::from(vec))),
    //         false => Ok(Value::List(vec)),
    //     }
    // }

    // #[inline]
    // fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    // where
    //     A: de::MapAccess<'de>,
    // {
    //     let mut m = BTreeMap::new();

    //     while let Some((k, v)) = map.next_entry()? {
    //         m.insert(k, v);
    //     }

    //     Ok(Value::Map(m))
    // }

    // #[inline]
    // fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    // where
    //     E: de::Error, 
    // {
    //     // Dec32, Dec64, Dec128
    //     todo!()
    // }

}

impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &'static [&'static str] = &[
            "Null", "Bool", "Ubyte", "Ushort", "Uint", "Ulong",
            "Byte", "Short", "Int", "Long", "Float", "Double",
            "Decimal32", "Decimal64", "Decimal128", "Char", "Timestamp",
            "Uuid", "Binary", "String", "Symbol", "List", "Map", "Array"
        ];
        deserializer.deserialize_enum(VALUE, VARIANTS, Visitor { })
    }
}

