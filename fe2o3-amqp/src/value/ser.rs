use serde::ser::Serialize;

use super::Value;

impl Serialize for Value {
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
