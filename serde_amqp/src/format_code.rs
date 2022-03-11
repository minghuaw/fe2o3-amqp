//! Encoding codes of AMQP types

use std::{convert::TryFrom, fmt::Display};

use crate::error::Error;

/// Encoding code for different types
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
#[repr(u8)]
pub enum EncodingCodes {
    DescribedType = 0x00 as u8,

    Null = 0x40,

    Boolean = 0x56,
    BooleanTrue = 0x41,
    BooleanFalse = 0x42,

    /// u8
    UByte = 0x50,

    /// u16
    UShort = 0x60,

    /// u32
    UInt = 0x70,
    SmallUint = 0x52,
    Uint0 = 0x43,

    /// u64
    ULong = 0x80,
    SmallUlong = 0x53,
    Ulong0 = 0x44,

    /// i8
    Byte = 0x51,

    /// i16
    Short = 0x61,

    ///i32
    Int = 0x71,
    SmallInt = 0x54,

    /// i64
    Long = 0x81,
    SmallLong = 0x55,

    /// f32
    Float = 0x72,

    /// f64
    Double = 0x82,

    Decimal32 = 0x74,

    Decimal64 = 0x84,

    Decimal128 = 0x94,

    Char = 0x73,

    Timestamp = 0x83,

    Uuid = 0x98,

    // Binary
    VBin8 = 0xa0,
    VBin32 = 0xb0,

    // String
    Str8 = 0xa1,
    Str32 = 0xb1,

    // A special version of String
    Sym8 = 0xa3,
    Sym32 = 0xb3,

    List0 = 0x45,
    List8 = 0xc0,
    List32 = 0xd0,

    Map8 = 0xc1,
    Map32 = 0xd1,

    Array8 = 0xe0,
    Array32 = 0xf0,
}

impl Display for EncodingCodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}:0x{:x}", self, self.clone() as u8)
    }
}

impl TryFrom<u8> for EncodingCodes {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            0x00 => EncodingCodes::DescribedType,

            0x40 => EncodingCodes::Null,

            0x56 => EncodingCodes::Boolean,
            0x41 => EncodingCodes::BooleanTrue,
            0x42 => EncodingCodes::BooleanFalse,

            // u8
            0x50 => EncodingCodes::UByte,

            // u16
            0x60 => EncodingCodes::UShort,

            // u32
            0x70 => EncodingCodes::UInt,
            0x52 => EncodingCodes::SmallUint,
            0x43 => EncodingCodes::Uint0,

            // u64
            0x80 => EncodingCodes::ULong,
            0x53 => EncodingCodes::SmallUlong,
            0x44 => EncodingCodes::Ulong0,

            // i8
            0x51 => EncodingCodes::Byte,

            // i16
            0x61 => EncodingCodes::Short,

            // i32
            0x71 => EncodingCodes::Int,
            0x54 => EncodingCodes::SmallInt,

            // i64
            0x81 => EncodingCodes::Long,
            0x55 => EncodingCodes::SmallLong,

            // f32
            0x72 => EncodingCodes::Float,

            // f64
            0x82 => EncodingCodes::Double,

            0x74 => EncodingCodes::Decimal32,
            0x84 => EncodingCodes::Decimal64,
            0x94 => EncodingCodes::Decimal128,

            0x73 => EncodingCodes::Char,

            0x83 => EncodingCodes::Timestamp,

            0x98 => EncodingCodes::Uuid,

            0xa0 => EncodingCodes::VBin8,
            0xb0 => EncodingCodes::VBin32,

            0xa1 => EncodingCodes::Str8,
            0xb1 => EncodingCodes::Str32,

            0xa3 => EncodingCodes::Sym8,
            0xb3 => EncodingCodes::Sym32,

            0x45 => EncodingCodes::List0,
            0xc0 => EncodingCodes::List8,
            0xd0 => EncodingCodes::List32,

            0xc1 => EncodingCodes::Map8,
            0xd1 => EncodingCodes::Map32,

            0xe0 => EncodingCodes::Array8,
            0xf0 => EncodingCodes::Array32,

            _ => return Err(Error::InvalidFormatCode),
        };

        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn size_of_encoding_codes() {
        let s = std::mem::size_of::<super::EncodingCodes>();
        println!("{}", s);
    }

    #[test]
    fn debug_encoding_codes() {
        let code = super::EncodingCodes::Null;
        println!("0x{:x}", code.clone() as u8);
        assert_eq!(code as u8, 0x40);
    }

    #[test]
    fn print_encoding_codes() {
        let code = super::EncodingCodes::Boolean;
        println!("{}", code);
    }
}
