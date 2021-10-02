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

impl<'de> de::Deserialize<'de> for SimpleValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Value::deserialize(deserializer)?
            .try_into()
            .map_err(|_| de::Error::custom("Invalid value for SimpleValue"))
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
