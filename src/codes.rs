//! Encoding codes of AMQP types
#[repr(u8)]
pub enum EncodingCodes {
    DescribedTypeIndicator = 0x00 as u8,
    
    Null = 0x40,
    
    Boolean = 0x56,
    BooleanTrue = 0x41,
    BooleanFalse = 0x42,

    UByte = 0x50,

    UShort = 0x60,

    UInt = 0x70,
    SmallUInt = 0x52,
    UInt0 = 0x43,

    ULong = 0x80,
    SmallULong = 0x53,
    ULong0 = 0x44,

    Byte = 0x51,

    Short = 0x61,

    Int = 0x71,
    SmallInt = 0x54,

    Long = 0x81,
    SmallLong = 0x55,

    Float = 0x72,

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

#[cfg(test)]
mod tests {
    #[test]
    fn size_of_encoding_codes() {
        let s = std::mem::size_of::<super::EncodingCodes>();
        println!("{}", s);
    }
}
