use std::convert::TryFrom;

use crate::format_code::EncodingCodes;

/// Offset of List in other implementations
/// The two implementations below do not count the byte(s) taken by `size` itself
/// 1. amqpnetlite: for List32
/// ```csharp
/// listSize = (totalBufferLength - 5); // totalBufferLength includes format code
/// ```
/// 2. qpid-protonj2: for List32
/// ```java
/// buffer.setInt(startIndex, endIndex - startIndex - 4); // startIndex is the byte after format code, endIndex is the end of the buffer
/// ```
/// offset includes 1 byte of `count`
pub const OFFSET_LIST8: usize = 1;
/// offset includes 4 byte of `count`
pub const OFFSET_LIST32: usize = 4;

/// offset includes 1 byte of `count`
pub const OFFSET_MAP8: usize = 1;
/// offset includes 4 byte of `count`
pub const OFFSET_MAP32: usize = 4;

/// offset includes 1 byte of `count` and 1 byte of element format code
pub const OFFSET_ARRAY8: usize = 2;
/// offset includes 4 bytes of `count` and 1 byte of element format code
pub const OFFSET_ARRAY32: usize = 5;

pub enum Category {
    Fixed(usize),
    Variable(usize),
    Compound(usize),
    Array(usize),
}

pub struct IsDescribed;

impl TryFrom<EncodingCodes> for Category {
    type Error = IsDescribed;

    fn try_from(value: EncodingCodes) -> Result<Self, Self::Error> {
        let value = match value {
            EncodingCodes::DescribedType => return Err(IsDescribed),

            EncodingCodes::Null => Category::Fixed(0),

            EncodingCodes::Boolean => Category::Fixed(1),
            EncodingCodes::BooleanTrue => Category::Fixed(0),
            EncodingCodes::BooleanFalse => Category::Fixed(0),

            // u8
            EncodingCodes::Ubyte => Category::Fixed(1),

            // u16
            EncodingCodes::Ushort => Category::Fixed(2),

            // u32
            EncodingCodes::Uint => Category::Fixed(4),
            EncodingCodes::SmallUint => Category::Fixed(1),
            EncodingCodes::Uint0 => Category::Fixed(0),

            // u64
            EncodingCodes::Ulong => Category::Fixed(8),
            EncodingCodes::SmallUlong => Category::Fixed(1),
            EncodingCodes::Ulong0 => Category::Fixed(0),

            // i8
            EncodingCodes::Byte => Category::Fixed(1),

            // i16
            EncodingCodes::Short => Category::Fixed(2),

            // i32
            EncodingCodes::Int => Category::Fixed(4),
            EncodingCodes::SmallInt => Category::Fixed(1),

            // i64
            EncodingCodes::Long => Category::Fixed(8),
            EncodingCodes::SmallLong => Category::Fixed(1),

            // f32
            EncodingCodes::Float => Category::Fixed(4),

            // f64
            EncodingCodes::Double => Category::Fixed(8),

            EncodingCodes::Decimal32 => Category::Fixed(4),
            EncodingCodes::Decimal64 => Category::Fixed(8),
            EncodingCodes::Decimal128 => Category::Fixed(16),

            EncodingCodes::Char => Category::Fixed(4),

            EncodingCodes::Timestamp => Category::Fixed(8),

            EncodingCodes::Uuid => Category::Fixed(16),

            EncodingCodes::Vbin8 => Category::Variable(1),
            EncodingCodes::Vbin32 => Category::Variable(4),

            EncodingCodes::Str8 => Category::Variable(1),
            EncodingCodes::Str32 => Category::Variable(4),

            EncodingCodes::Sym8 => Category::Variable(1),
            EncodingCodes::Sym32 => Category::Variable(4),

            EncodingCodes::List0 => Category::Fixed(0),
            EncodingCodes::List8 => Category::Compound(1),
            EncodingCodes::List32 => Category::Compound(4),

            EncodingCodes::Map8 => Category::Compound(1),
            EncodingCodes::Map32 => Category::Compound(4),

            EncodingCodes::Array8 => Category::Array(1),
            EncodingCodes::Array32 => Category::Array(4),
        };

        Ok(value)
    }
}
