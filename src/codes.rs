//! Encoding codes of AMQP types

use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum EncodingCodes {
    DescribedTypeIndicator = 0x00 as u8,
    
    Null = 0x40,
    
    Boolean = 0x56,
    BooleanTrue = 0x41,
    BooleanFalse = 0x42,

    /// u8
    Ubyte = 0x50,

    /// u16
    Ushort = 0x60,

    /// u32
    Uint = 0x70,
    SmallUint = 0x52,
    Uint0 = 0x43,

    /// u64
    Ulong = 0x80,
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

    VBin8 = 0xa0,
    VBin32 = 0xb0,

    Str8 = 0xa1,
    Str32 = 0xb1,

    Sym8 = 0xa3,
    Sym32 = 0xb3,

    List0 = 0x45,
    List8 = 0xc0,
    List32 = 0xd0,

    Map8 = 0xc1,
    Map32 = 0xd1,

    Array8 = 0xe0,
    Array32 = 0xf0
}

impl Display for EncodingCodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}:0x{:x}", self, self.clone() as u8)
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
