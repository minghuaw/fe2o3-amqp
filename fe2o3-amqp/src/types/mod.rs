mod array;
mod decimal;
mod descriptor;
mod described;
mod symbol;
mod timestamp;
mod uuid;

pub use array::*;
pub use decimal::*;
pub use descriptor::*;
pub use described::*;
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
