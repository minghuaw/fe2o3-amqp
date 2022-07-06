//! Simple values. A subset of the primitive types.

use super::*;

/// A subset of `Value`
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimpleValue {
    /// Indicates an empty value
    ///
    /// encoding code = 0x40,
    /// category = fixed, width = 0,
    /// label = "the null value"
    Null,

    /// Represents a true or false value
    ///
    /// encoding code = 0x56
    /// category = fixed, width = 1
    /// label = "boolean with the octet 0x00 being false and octet 0x01 being true"
    ///
    /// encoding name = "true", encoding code = 0x41
    /// category = fixed, width = 0
    /// label = "the boolean value true"
    ///
    /// encoding name = "false", encoding code = 0x42
    /// category = fixed, width = 0
    /// label = "the boolean value false"
    Bool(Boolean),

    /// Integer in the range 0 to 2^8-1 inclusive
    ///
    /// encoding code = 0x50,
    /// category = fixed, width = 1
    /// label = "8-bit unsigned integer"
    UByte(UByte),

    /// Integer in the range 0 to 2^16-1 inclusive
    ///
    /// encoding code = 0x60,
    /// category = fixed, width = 2
    /// label = "16-bit unsigned integer in network byte order"
    /// (AKA. Big-Endian, rust uses BigEndian by default)
    UShort(UShort),

    /// Integer in the range 0 to 2^32-1 inclusive
    ///
    /// encoding code = 0x70,
    /// category = fixed, width = 4
    /// label = "32-bit unsigned integer in network byte order"
    /// (AKA. Big-Endian, rust uses BigEndian by default)
    ///
    /// encoding name = "smalluint", encoding code = 0x52
    /// category = fixed, width = 1
    /// label = "unsigned integer value in the range 0 to 255 inclusive"
    ///
    /// encoding name = "uint0", encoding code = 0x43
    /// category = fixed, width = 0
    /// label = "the uint value 0"
    UInt(UInt),

    /// Integer in the range 0 to 2^64-1 inclusive
    ///
    /// encoding code = 0x80,
    /// category = fixed, width = 8
    /// label = "64-bit unsigned integer in network byte order"
    /// (AKA. Big-Endian, rust uses BigEndian by default)
    ///
    /// encoding name = "smallulong", encoding code = 0x53
    /// category = fixed, width = 1
    /// label = "unsigned long value in the range 0 to 255 inclusive"
    ///
    /// encoding name = "ulong0", encoding code = 0x44
    /// category = fixed, width = 0
    /// label = "the ulong value 0"
    ULong(ULong),

    /// Integer in the range -(2^7) to 2^7-1 inclusive
    ///
    /// encoding code = 0x51,
    /// category = fixed, width = 1
    /// label = "8-bit two's-complement integer"
    Byte(Byte),

    /// Integer in the range -(2^15) to 2^15-1 inclusive
    ///
    /// encoding code = 0x61,
    /// category = fixed, width = 2
    /// label = "16-bit two’s-complement integer in network byte order"
    Short(Short),

    /// Integer in the range -(2^31) to 2^31-1 inclusive
    ///
    /// encoding code = 0x71,
    /// category = fixed, width = 4
    /// label = "32-bit two’s-complement integer in network byte order"
    ///
    /// encoding name = "smallint", encoding code = 0x54
    /// category = fixed, width = 1
    /// label = "8-bit two’s-complement integer"
    Int(Int),

    /// Integer in the range -(2^63) to 2^63-1 inclusive
    ///
    /// encoding code = 0x81,
    /// category = fixed, width = 8
    /// label = "64-bit two’s-complement integer in network byte order"
    ///
    /// encoding name = "smalllong", encoding code = 0x55
    /// category = fixed, width = 1
    /// label = "8-bit two’s-complement integer"
    Long(Long),

    /// 32-bit floating point number (IEEE 754-2008 binary32)
    ///
    /// encoding name = "ieee-754", encoding code = 0x72
    /// category = fixed, width = 4
    /// label = "IEEE 754-2008 binary32"
    Float(OrderedFloat<Float>),

    /// 64-bit floating point number (IEEE 754-2008 binary64).
    ///
    /// encoding name = "ieee-754", encoding code = 0x82
    /// category = fixed, width = 8
    /// label = "IEEE 754-2008 binary64"
    Double(OrderedFloat<Double>),

    /// 32-bit decimal number (IEEE 754-2008 decimal32).
    ///
    /// encoding name = "ieee-754", encoding code = 0x74
    /// category = fixed, width = 4
    /// label = "IEEE 754-2008 decimal32 using the Binary Integer Decimal encoding"
    Decimal32(Dec32),

    /// 64-bit decimal number (IEEE 754-2008 decimal64).
    ///
    /// encoding name = "ieee-754", encoding code = 0x84
    /// category = fixed, width = 8
    /// label = "IEEE 754-2008 decimal64 using the Binary Integer Decimal encoding"
    Decimal64(Dec64),

    /// 128-bit decimal number (IEEE 754-2008 decimal128).
    ///
    /// encoding name = "ieee-754", encoding code = 0x94
    /// category = fixed, width = 16
    /// label = "IEEE 754-2008 decimal128 using the Binary Integer Decimal encoding"
    Decimal128(Dec128),

    /// A single Unicode character
    ///
    /// encoding name = "utf32", encoding code = 0x73
    /// category = fixed, width = 4,
    /// label = "a UTF-32BE encoded Unicode character"
    Char(Char),

    /// An absolute point in time
    ///
    /// encoding name = "ms64", code = 0x83,
    /// category = fixed, width = 8
    /// label = "64-bit two’s-complement integer representing milliseconds since the unix epoch"
    Timestamp(Timestamp),

    /// An absolute point in time
    ///
    /// encoding name = "ms64", code = 0x83,
    /// category = fixed, width = 8
    /// label = "64-bit two’s-complement integer representing milliseconds since the unix epoch"

    /// A universally unique identifier as defined by RFC-4122 in section 4.1.2
    ///
    /// encoding code = 0x98,
    /// category = fixed, width = 16,
    /// label="UUID as defined in section 4.1.2 of RFC-4122"
    Uuid(Uuid),

    /// A sequence of octets.
    ///
    /// encoding name = "vbin8", encoding code = 0xa0
    /// category = variable, width = 1
    /// label = "up to 2^8 - 1 octets of binary data"
    ///
    /// encoding name = "vbin32", encoding code = 0xb0,
    /// category = variable, width = 4,
    /// label="up to 2^32 - 1 octets of binary data"
    Binary(ByteBuf),

    /// A sequence of Unicode characters.
    ///
    /// encoding name = "str8-utf8", encoding code = 0xa1,
    /// category = variable, width = 1
    /// label = "up to 2^8 - 1 octets worth of UTF-8 Unicode (with no byte order mark)"
    ///
    /// encoding name = "str32-utf8", encoding code = 0xb1
    /// category = variable, width = 4
    /// label="up to 2^32 - 1 octets worth of UTF-8 Unicode (with no byte order mark)"
    String(String),

    /// Symbolic values from a constrained domain.
    ///
    /// encoding name = "sym8", encoding code = 0xa3,
    /// category = variable, width = 1
    /// label="up to 2^8 - 1 seven bit ASCII characters representing a symbolic value"
    ///
    /// encoding name = "sym32", encoding code = 0xb3
    /// category = variable, width = 4
    /// label="up to 2^32 - 1 seven bit ASCII characters representing a symbolic value"
    ///
    /// Symbols are values from a constrained domain.
    /// Although the set of possible domains is open-ended,
    /// typically the both number and size of symbols in use for any
    /// given application will be small, e.g. small enough that it is reasonable
    /// to cache all the distinct values. Symbols are encoded as ASCII characters ASCII.
    Symbol(Symbol),
}

impl Default for SimpleValue {
    fn default() -> Self {
        SimpleValue::Null
    }
}

impl SimpleValue {
    /// Get the format code of the type
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
            SimpleValue::String(v) => serializer.serialize_str(v),
            SimpleValue::Symbol(v) => v.serialize(serializer),
        }
    }
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
    type Error = serde_amqp::error::Error;

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
                return Err(serde_amqp::error::Error::InvalidValue)
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

macro_rules! impl_from_for_simple_value {
    ($variant:ident, $variant_ty:ty) => {
        impl From<$variant_ty> for SimpleValue {
            fn from(val: $variant_ty) -> Self {
                Self::$variant(val)
            }
        }
    };

    ($($variant:ident, $variant_ty:ty),*) => {
        $(impl_from_for_simple_value!($variant, $variant_ty);)*
    }
}

impl_from_for_simple_value! {
    Bool, bool,
    UByte, u8,
    UShort, u16,
    UInt, u32,
    ULong, u64,
    Byte, i8,
    Short, i16,
    Int, i32,
    Long, i64,
    Float, OrderedFloat<f32>,
    Double, OrderedFloat<f64>,
    Decimal32, Dec32,
    Decimal64, Dec64,
    Decimal128, Dec128,
    Char, char,
    Timestamp, Timestamp,
    Uuid, Uuid,
    Binary, ByteBuf,
    String, String,
    Symbol, Symbol
}

impl From<f32> for SimpleValue {
    fn from(val: f32) -> Self {
        Self::Float(OrderedFloat::from(val))
    }
}

impl From<f64> for SimpleValue {
    fn from(val: f64) -> Self {
        Self::Double(OrderedFloat::from(val))
    }
}

impl From<&str> for SimpleValue {
    fn from(val: &str) -> Self {
        Self::String(val.to_string())
    }
}
