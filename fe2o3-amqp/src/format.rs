use std::convert::TryFrom;

use crate::{constructor::EncodingCodes, error::Error};

pub enum Category {
    Fixed(Width),
    Variable(Width),
    Compound(Width),
    Array(Width),
}

#[repr(u8)]
pub enum Width {
    Zero = 0,
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
    Sixteen = 16,
}

impl TryFrom<EncodingCodes> for Category {
    type Error = Error;
    
    fn try_from(value: EncodingCodes) -> Result<Self, Self::Error> {
        let value = match value {
            EncodingCodes::DescribedType => return Err(Error::IsDescribedType),
        
            EncodingCodes::Null => Category::Fixed(Width::Zero),
        
            EncodingCodes::Boolean => Category::Fixed(Width::One),
            EncodingCodes::BooleanTrue => Category::Fixed(Width::Zero),
            EncodingCodes::BooleanFalse => Category::Fixed(Width::Zero),

            // u8
             EncodingCodes::Ubyte => Category::Fixed(Width::One),

            // u16
             EncodingCodes::Ushort => Category::Fixed(Width::Two),

            // u32
             EncodingCodes::Uint => Category::Fixed(Width::Four),
             EncodingCodes::SmallUint => Category::Fixed(Width::One),
             EncodingCodes::Uint0 => Category::Fixed(Width::Zero),

            // u64
             EncodingCodes::Ulong => Category::Fixed(Width::Eight),
             EncodingCodes::SmallUlong => Category::Fixed(Width::One),
             EncodingCodes::Ulong0 => Category::Fixed(Width::Zero),

            // i8
             EncodingCodes::Byte => Category::Fixed(Width::One),

            // i16 
             EncodingCodes::Short => Category::Fixed(Width::Two),

            // i32
             EncodingCodes::Int => Category::Fixed(Width::Four),
             EncodingCodes::SmallInt => Category::Fixed(Width::One),

            // i64
             EncodingCodes::Long => Category::Fixed(Width::Eight),
             EncodingCodes::SmallLong => Category::Fixed(Width::One),

            // f32
             EncodingCodes::Float => Category::Fixed(Width::Four),

            // f64
             EncodingCodes::Double => Category::Fixed(Width::Eight),

             EncodingCodes::Decimal32 => Category::Fixed(Width::Four),
             EncodingCodes::Decimal64 => Category::Fixed(Width::Eight),
             EncodingCodes::Decimal128 => Category::Fixed(Width::Sixteen),

             EncodingCodes::Char => Category::Fixed(Width::Four),
            
             EncodingCodes::Timestamp => Category::Fixed(Width::Eight),

             EncodingCodes::Uuid => Category::Fixed(Width::Sixteen),

            EncodingCodes::VBin8 => Category::Variable(Width::One),
            EncodingCodes::VBin32 => Category::Variable(Width::Four),

            EncodingCodes::Str8 => Category::Variable(Width::One),
            EncodingCodes::Str32 => Category::Variable(Width::Four),

            EncodingCodes::Sym8 => Category::Variable(Width::One),
            EncodingCodes::Sym32 => Category::Variable(Width::Four),

            EncodingCodes::List0 => Category::Compound(Width::Zero),
            EncodingCodes::List8 => Category::Compound(Width::One),
            EncodingCodes::List32 => Category::Compound(Width::Four),

            EncodingCodes::Map8 => Category::Compound(Width::One),
            EncodingCodes::Map32 => Category::Compound(Width::Four),
            
            EncodingCodes::Array8 => Category::Array(Width::One),
            EncodingCodes::Array32 => Category::Array(Width::Four)
        };

        Ok(value)
    }
}
