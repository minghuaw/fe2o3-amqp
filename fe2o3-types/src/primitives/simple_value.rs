use super::*;

/// A subset of `SimpleValue`
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimpleValue {
    Null,
    Bool(Boolean),
    UByte(UByte),
    UShort(UShort),
    UInt(UInt),
    ULong(ULong),
    Byte(Byte),
    Short(Short),
    Int(Int),
    Long(Long),
    Float(OrderedFloat<Float>),
    Double(OrderedFloat<Double>),
    Decimal32(Dec32),
    Decimal64(Dec64),
    Decimal128(Dec128),
    Char(Char),
    Timestamp(Timestamp),
    Uuid(Uuid),
    Binary(ByteBuf),
    String(String),
    Symbol(Symbol),
}

impl Default for SimpleValue {
    fn default() -> Self {
        SimpleValue::Null
    }
}

impl SimpleValue {
    pub fn format_code(&self) -> u8 {
        let code = match *self {
            SimpleValue::Null => EncodingCodes::Null,
            SimpleValue::Bool(_) => EncodingCodes::Boolean,
            SimpleValue::UByte(_) => EncodingCodes::UByte,
            SimpleValue::UShort(_) => EncodingCodes::UShort,
            SimpleValue::UInt(_) => EncodingCodes::UInt,
            SimpleValue::ULong(_) => EncodingCodes::ULong,
            SimpleValue::Byte(_) => EncodingCodes::Byte,
            SimpleValue::Short(_) => EncodingCodes::Short,
            SimpleValue::Int(_) => EncodingCodes::Int,
            SimpleValue::Long(_) => EncodingCodes::Long,
            SimpleValue::Float(_) => EncodingCodes::Float,
            SimpleValue::Double(_) => EncodingCodes::Double,
            SimpleValue::Decimal32(_) => EncodingCodes::Decimal32,
            SimpleValue::Decimal64(_) => EncodingCodes::Decimal64,
            SimpleValue::Decimal128(_) => EncodingCodes::Decimal128,
            SimpleValue::Char(_) => EncodingCodes::Char,
            SimpleValue::Timestamp(_) => EncodingCodes::Timestamp,
            SimpleValue::Uuid(_) => EncodingCodes::Uuid,
            SimpleValue::Binary(_) => EncodingCodes::VBin32,
            SimpleValue::String(_) => EncodingCodes::Str32,
            SimpleValue::Symbol(_) => EncodingCodes::Sym32,
        };
        code as u8
    }
}

impl ser::Serialize for SimpleValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            SimpleValue::Null => serializer.serialize_unit(),
            SimpleValue::Bool(v) => serializer.serialize_bool(*v),
            SimpleValue::UByte(v) => serializer.serialize_u8(*v),
            SimpleValue::UShort(v) => serializer.serialize_u16(*v),
            SimpleValue::UInt(v) => serializer.serialize_u32(*v),
            SimpleValue::ULong(v) => serializer.serialize_u64(*v),
            SimpleValue::Byte(v) => serializer.serialize_i8(*v),
            SimpleValue::Short(v) => serializer.serialize_i16(*v),
            SimpleValue::Int(v) => serializer.serialize_i32(*v),
            SimpleValue::Long(v) => serializer.serialize_i64(*v),
            SimpleValue::Float(v) => serializer.serialize_f32(v.into_inner()),
            SimpleValue::Double(v) => serializer.serialize_f64(v.into_inner()),
            SimpleValue::Decimal32(v) => v.serialize(serializer),
            SimpleValue::Decimal64(v) => v.serialize(serializer),
            SimpleValue::Decimal128(v) => v.serialize(serializer),
            SimpleValue::Char(v) => serializer.serialize_char(*v),
            SimpleValue::Timestamp(v) => v.serialize(serializer),
            SimpleValue::Uuid(v) => v.serialize(serializer),
            SimpleValue::Binary(v) => serializer.serialize_bytes(v.as_slice()),
            SimpleValue::String(v) => serializer.serialize_str(&v),
            SimpleValue::Symbol(v) => v.serialize(serializer),
        }
    }
}

enum Field {
    Null,
    Bool,
    UByte,
    UShort,
    UInt,
    ULong,
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
}

struct FieldVisitor {}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("variant identifier")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let field = match v
            .try_into()
            .map_err(|_| de::Error::custom("Failed to convert u8 to format code"))?
        {
            EncodingCodes::Null => Field::Null,
            EncodingCodes::Boolean | EncodingCodes::BooleanFalse | EncodingCodes::BooleanTrue => {
                Field::Bool
            }
            EncodingCodes::UByte => Field::UByte,
            EncodingCodes::UShort => Field::UShort,
            EncodingCodes::UInt | EncodingCodes::Uint0 | EncodingCodes::SmallUint => Field::UInt,
            EncodingCodes::ULong | EncodingCodes::Ulong0 | EncodingCodes::SmallUlong => {
                Field::ULong
            }
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

            // unsupported types
            EncodingCodes::List0
            | EncodingCodes::List32
            | EncodingCodes::List8
            | EncodingCodes::Map32
            | EncodingCodes::Map8
            | EncodingCodes::Array32
            | EncodingCodes::Array8
            | EncodingCodes::DescribedType => {
                return Err(de::Error::custom("Only simple types are supported"))
            }
        };
        Ok(field)
    }
}

impl<'de> de::Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_identifier(FieldVisitor {})
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = SimpleValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum SimpleValue")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        use de::VariantAccess;
        let (val, de) = data.variant()?;

        match val {
            Field::Null => {
                let _: () = de.newtype_variant()?;
                Ok(SimpleValue::Null)
            }
            Field::Bool => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Bool(val))
            }
            Field::UByte => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::UByte(val))
            }
            Field::UShort => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::UShort(val))
            }
            Field::UInt => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::UInt(val))
            }
            Field::ULong => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::ULong(val))
            }
            Field::Byte => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Byte(val))
            }
            Field::Short => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Short(val))
            }
            Field::Int => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Int(val))
            }
            Field::Long => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Long(val))
            }
            Field::Float => {
                let val: f32 = de.newtype_variant()?;
                Ok(SimpleValue::Float(OrderedFloat::from(val)))
            }
            Field::Double => {
                let val: f64 = de.newtype_variant()?;
                Ok(SimpleValue::Double(OrderedFloat::from(val)))
            }
            Field::Decimal32 => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Decimal32(val))
            }
            Field::Decimal64 => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Decimal64(val))
            }
            Field::Decimal128 => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Decimal128(val))
            }
            Field::Char => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Char(val))
            }
            Field::Timestamp => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Timestamp(val))
            }
            Field::Uuid => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Uuid(val))
            }
            Field::Binary => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Binary(val))
            }
            Field::String => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::String(val))
            }
            Field::Symbol => {
                let val = de.newtype_variant()?;
                Ok(SimpleValue::Symbol(val))
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for SimpleValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &'static [&'static str] = &[
            "Null",
            "Bool",
            "UByte",
            "UShort",
            "UInt",
            "ULong",
            "Byte",
            "Short",
            "Int",
            "Long",
            "Float",
            "Double",
            "Decimal32",
            "Decimal64",
            "Decimal128",
            "Char",
            "Timestamp",
            "Uuid",
            "Binary",
            "String",
            "Symbol",
        ];
        deserializer.deserialize_enum(VALUE, VARIANTS, Visitor {})
    }
}

impl TryFrom<Value> for SimpleValue {
    type Error = fe2o3_amqp::error::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let val = match value {
            Value::Null => SimpleValue::Null,
            Value::Bool(v) => SimpleValue::Bool(v),
            Value::UByte(v) => SimpleValue::UByte(v),
            Value::UShort(v) => SimpleValue::UShort(v),
            Value::UInt(v) => SimpleValue::UInt(v),
            Value::ULong(v) => SimpleValue::ULong(v),
            Value::Byte(v) => SimpleValue::Byte(v),
            Value::Short(v) => SimpleValue::Short(v),
            Value::Int(v) => SimpleValue::Int(v),
            Value::Long(v) => SimpleValue::Long(v),
            Value::Float(v) => SimpleValue::Float(v),
            Value::Double(v) => SimpleValue::Double(v),
            Value::Decimal32(v) => SimpleValue::Decimal32(v),
            Value::Decimal64(v) => SimpleValue::Decimal64(v),
            Value::Decimal128(v) => SimpleValue::Decimal128(v),
            Value::Char(v) => SimpleValue::Char(v),
            Value::Timestamp(v) => SimpleValue::Timestamp(v),
            Value::Uuid(v) => SimpleValue::Uuid(v),
            Value::Binary(v) => SimpleValue::Binary(v),
            Value::String(v) => SimpleValue::String(v),
            Value::Symbol(v) => SimpleValue::Symbol(v),
            Value::List(_) | Value::Map(_) | Value::Array(_) | Value::Described(_) => {
                return Err(fe2o3_amqp::error::Error::InvalidValue)
            }
        };
        Ok(val)
    }
}

impl From<SimpleValue> for Value {
    fn from(value: SimpleValue) -> Self {
        match value {
            SimpleValue::Null => Value::Null,
            SimpleValue::Bool(v) => Value::Bool(v),
            SimpleValue::UByte(v) => Value::UByte(v),
            SimpleValue::UShort(v) => Value::UShort(v),
            SimpleValue::UInt(v) => Value::UInt(v),
            SimpleValue::ULong(v) => Value::ULong(v),
            SimpleValue::Byte(v) => Value::Byte(v),
            SimpleValue::Short(v) => Value::Short(v),
            SimpleValue::Int(v) => Value::Int(v),
            SimpleValue::Long(v) => Value::Long(v),
            SimpleValue::Float(v) => Value::Float(v),
            SimpleValue::Double(v) => Value::Double(v),
            SimpleValue::Decimal32(v) => Value::Decimal32(v),
            SimpleValue::Decimal64(v) => Value::Decimal64(v),
            SimpleValue::Decimal128(v) => Value::Decimal128(v),
            SimpleValue::Char(v) => Value::Char(v),
            SimpleValue::Timestamp(v) => Value::Timestamp(v),
            SimpleValue::Uuid(v) => Value::Uuid(v),
            SimpleValue::Binary(v) => Value::Binary(v),
            SimpleValue::String(v) => Value::String(v),
            SimpleValue::Symbol(v) => Value::Symbol(v),
        }
    }
}
