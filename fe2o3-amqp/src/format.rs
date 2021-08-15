use std::convert::TryFrom;

use crate::constructor::EncodingCodes;

pub enum Category {
    Fixed(FixedWidth),
    Variable(VariableWidth),
    Compound(CompoundWidth),
    Array(ArrayWidth),
}

#[repr(u8)]
pub enum FixedWidth {
    Zero = 0,
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
    Sixteen = 16,
}

#[repr(u8)]
pub enum VariableWidth {
    One = 1,
    Four = 4,
}

#[repr(u8)]
pub enum CompoundWidth {
    One = 1,
    Four = 4,
}

#[repr(u8)]
pub enum ArrayWidth {
    One = 1,
    Four = 4
}
