use std::convert::TryFrom;

use crate::{error::Error, format_code::EncodingCodes};

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
    Fixed,
    Encoded,
}

// #[repr(u8)]
// pub enum FixedWidth {
//     Zero = 0,
//     One = 1,
//     Two = 2,
//     Four = 4,
//     Eight = 8,
//     Sixteen = 16,
// }

// #[repr(u8)]
// pub enum EncodedWidth {
//     Zero = 0,
//     One = 1,
//     Four = 4,
// }

// #[repr(u8)]
// pub enum VariableWidth {
//     One = 1,
//     Four = 4,
// }

// #[repr(u8)]
// pub enum CompoundWidth {
//     Zero = 0,
//     One = 1,
//     Four = 4,
// }

// #[repr(u8)]
// pub enum ArrayWidth {
//     One = 1,
//     Four = 4,
// }

impl TryFrom<EncodingCodes> for Category {
    type Error = Error;

    fn try_from(value: EncodingCodes) -> Result<Self, Self::Error> {
        let value = match value {
            EncodingCodes::DescribedType => return Err(Error::IsDescribedType),

            EncodingCodes::Null => Category::Fixed,

            EncodingCodes::Boolean => Category::Fixed,
            EncodingCodes::BooleanTrue => Category::Fixed,
            EncodingCodes::BooleanFalse => Category::Fixed,

            // u8
            EncodingCodes::Ubyte => Category::Fixed,

            // u16
            EncodingCodes::Ushort => Category::Fixed,

            // u32
            EncodingCodes::Uint => Category::Fixed,
            EncodingCodes::SmallUint => Category::Fixed,
            EncodingCodes::Uint0 => Category::Fixed,

            // u64
            EncodingCodes::Ulong => Category::Fixed,
            EncodingCodes::SmallUlong => Category::Fixed,
            EncodingCodes::Ulong0 => Category::Fixed,

            // i8
            EncodingCodes::Byte => Category::Fixed,

            // i16
            EncodingCodes::Short => Category::Fixed,

            // i32
            EncodingCodes::Int => Category::Fixed,
            EncodingCodes::SmallInt => Category::Fixed,

            // i64
            EncodingCodes::Long => Category::Fixed,
            EncodingCodes::SmallLong => Category::Fixed,

            // f32
            EncodingCodes::Float => Category::Fixed,

            // f64
            EncodingCodes::Double => Category::Fixed,

            EncodingCodes::Decimal32 => Category::Fixed,
            EncodingCodes::Decimal64 => Category::Fixed,
            EncodingCodes::Decimal128 => Category::Fixed,

            EncodingCodes::Char => Category::Fixed,

            EncodingCodes::Timestamp => Category::Fixed,

            EncodingCodes::Uuid => Category::Fixed,

            EncodingCodes::Vbin8 => Category::Encoded,
            EncodingCodes::Vbin32 => Category::Encoded,

            EncodingCodes::Str8 => Category::Encoded,
            EncodingCodes::Str32 => Category::Encoded,

            EncodingCodes::Sym8 => Category::Encoded,
            EncodingCodes::Sym32 => Category::Encoded,

            EncodingCodes::List0 => Category::Encoded,
            EncodingCodes::List8 => Category::Encoded,
            EncodingCodes::List32 => Category::Encoded,

            EncodingCodes::Map8 => Category::Encoded,
            EncodingCodes::Map32 => Category::Encoded,

            EncodingCodes::Array8 => Category::Encoded,
            EncodingCodes::Array32 => Category::Encoded,
        };

        Ok(value)
    }
}
