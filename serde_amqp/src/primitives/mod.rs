//! Definition of the primitive types

mod array;
mod decimal;
mod symbol;
mod timestamp;
mod uuid;

// to avoid ambiguity
pub use crate::primitives::array::*;
pub use crate::primitives::decimal::*;
pub use crate::primitives::symbol::*;
pub use crate::primitives::timestamp::*;
pub use crate::primitives::uuid::*;

// Alias for the primitive types to match those in the spec
use serde_bytes::ByteBuf;

/// Represents a true or false value
///
/// encoding code = 0x56
/// category = fixed, width = 1
/// label = "boolean with the octet 0x00 being false and octet 0x01 being true"
///
/// encoding name = "true", encoding code = 0x41
/// category = fixed, width = 0
/// label = "the boolean value true"
///
/// encoding name = "false", encoding code = 0x42
/// category = fixed, width = 0
/// label = "the boolean value false"
pub type Boolean = bool;

/// Integer in the range 0 to 2^8-1 inclusive
///
/// encoding code = 0x50,
/// category = fixed, width = 1
/// label = "8-bit unsigned integer"
pub type UByte = u8;

/// Integer in the range 0 to 2^16-1 inclusive
///
/// encoding code = 0x60,
/// category = fixed, width = 2
/// label = "16-bit unsigned integer in network byte order"
/// (AKA. Big-Endian, rust uses BigEndian by default)
pub type UShort = u16;

/// Integer in the range 0 to 2^32-1 inclusive
///
/// encoding code = 0x70,
/// category = fixed, width = 4
/// label = "32-bit unsigned integer in network byte order"
/// (AKA. Big-Endian, rust uses BigEndian by default)
///
/// encoding name = "smalluint", encoding code = 0x52
/// category = fixed, width = 1
/// label = "unsigned integer value in the range 0 to 255 inclusive"
///
/// encoding name = "uint0", encoding code = 0x43
/// category = fixed, width = 0
/// label = "the uint value 0"
pub type UInt = u32;

/// Integer in the range 0 to 2^64-1 inclusive
///
/// encoding code = 0x80,
/// category = fixed, width = 8
/// label = "64-bit unsigned integer in network byte order"
/// (AKA. Big-Endian, rust uses BigEndian by default)
///
/// encoding name = "smallulong", encoding code = 0x53
/// category = fixed, width = 1
/// label = "unsigned long value in the range 0 to 255 inclusive"
///
/// encoding name = "ulong0", encoding code = 0x44
/// category = fixed, width = 0
/// label = "the ulong value 0"
pub type ULong = u64;

/// Integer in the range -(2^7) to 2^7-1 inclusive
///
/// encoding code = 0x51,
/// category = fixed, width = 1
/// label = "8-bit two's-complement integer"
pub type Byte = i8;

/// Integer in the range -(2^15) to 2^15-1 inclusive
///
/// encoding code = 0x61,
/// category = fixed, width = 2
/// label = "16-bit two’s-complement integer in network byte order"
pub type Short = i16;

/// Integer in the range -(2^31) to 2^31-1 inclusive
///
/// encoding code = 0x71,
/// category = fixed, width = 4
/// label = "32-bit two’s-complement integer in network byte order"
///
/// encoding name = "smallint", encoding code = 0x54
/// category = fixed, width = 1
/// label = "8-bit two’s-complement integer"
pub type Int = i32;

/// Integer in the range -(2^63) to 2^63-1 inclusive
///
/// encoding code = 0x81,
/// category = fixed, width = 8
/// label = "64-bit two’s-complement integer in network byte order"
///
/// encoding name = "smalllong", encoding code = 0x55
/// category = fixed, width = 1
/// label = "8-bit two’s-complement integer"
pub type Long = i64;

/// 32-bit floating point number (IEEE 754-2008 binary32)
///
/// encoding name = "ieee-754", encoding code = 0x72
/// category = fixed, width = 4
/// label = "IEEE 754-2008 binary32"
pub type Float = f32;

/// 64-bit floating point number (IEEE 754-2008 binary64).
///
/// encoding name = "ieee-754", encoding code = 0x82
/// category = fixed, width = 8
/// label = "IEEE 754-2008 binary64"
pub type Double = f64;

/// A single Unicode character
///
/// encoding name = "utf32", encoding code = 0x73
/// category = fixed, width = 4,
/// label = "a UTF-32BE encoded Unicode character"
pub type Char = char;

/// A sequence of octets.
///
/// encoding name = "vbin8", encoding code = 0xa0
/// category = variable, width = 1
/// label = "up to 2^8 - 1 octets of binary data"
///
/// encoding name = "vbin32", encoding code = 0xb0,
/// category = variable, width = 4,
/// label="up to 2^32 - 1 octets of binary data"
pub type Binary = ByteBuf;
