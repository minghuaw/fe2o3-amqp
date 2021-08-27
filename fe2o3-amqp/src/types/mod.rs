mod array;
mod decimal;
mod described;
mod descriptor;
mod symbol;
mod timestamp;
mod uuid;

pub use array::*;
pub use decimal::*;
pub use described::*;
pub use descriptor::*;
pub use symbol::*;
pub use timestamp::*;
pub use uuid::*;

// Alias for the primitive types to match those in the spec
use serde_bytes::ByteBuf;

pub type Boolean = bool;
pub type Ubyte = u8;
pub type Ushort = u16;
pub type Uint = u32;
pub type Ulong = u64;
pub type Byte = i8;
pub type Short = i16;
pub type Int = i32;
pub type Long = i64;
pub type Float = f32;
pub type Double = f64;
pub type Char = char;
pub type Binary = ByteBuf;

pub enum Type<T> {
    Described(Described<T>),
    NonDescribed(T)
}

impl<T> Type<T> {
    pub fn is_described(&self) -> bool {
        match self {
            Type::Described(_) => true,
            Type::NonDescribed(_) => false
        }
    }
}

impl<T> From<Described<T>> for Type<T> {
    fn from(d: Described<T>) -> Self {
        Self::Described(d)
    }
}

use crate::convert::IntoDescribed;