use ordered_float::OrderedFloat;
use serde_bytes::ByteBuf;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::types::{Dec128, Dec32, Dec64, Symbol, Timestamp};

pub const U32_MAX_AS_USIZE: usize = u32::MAX as usize;

/// Primitive type definitions
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    /// Indicates an empty value
    ///
    /// encoding code = 0x40,
    /// category = fixed, width = 0,
    /// label = "the null value"
    Null,

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
    Bool(bool),

    /// Integer in the range 0 to 2^8-1 inclusive
    ///
    /// encoding code = 0x50,
    /// category = fixed, width = 1
    /// label = "8-bit unsigned integer"
    Ubyte(u8),

    /// Integer in the range 0 to 2^16-1 inclusive
    ///
    /// encoding code = 0x60,
    /// category = fixed, width = 2
    /// label = "16-bit unsigned integer in network byte order"
    /// (AKA. Big-Endian, rust uses BigEndian by default)
    Ushort(u16),

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
    Uint(u32),

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
    Ulong(u64),

    /// Integer in the range -(2^7) to 2^7-1 inclusive
    ///
    /// encoding code = 0x51,
    /// category = fixed, width = 1
    /// label = "8-bit two's-complement integer"
    Byte(i8),

    /// Integer in the range -(2^15) to 2^15-1 inclusive
    ///
    /// encoding code = 0x61,
    /// category = fixed, width = 2
    /// label = "16-bit two’s-complement integer in network byte order"
    Short(i16),

    /// Integer in the range -(2^31) to 2^31-1 inclusive
    ///
    /// encoding code = 0x71,
    /// category = fixed, width = 4
    /// label = "32-bit two’s-complement integer in network byte order"
    ///
    /// encoding name = "smallint", encoding code = 0x54
    /// category = fixed, width = 1
    /// label = "8-bit two’s-complement integer"
    Int(i32),

    /// Integer in the range -(2^63) to 2^63-1 inclusive
    ///
    /// encoding code = 0x81,
    /// category = fixed, width = 8
    /// label = "64-bit two’s-complement integer in network byte order"
    ///
    /// encoding name = "smalllong", encoding code = 0x55
    /// category = fixed, width = 1
    /// label = "8-bit two’s-complement integer"
    Long(i64),

    /// 32-bit floating point number (IEEE 754-2008 binary32)
    ///
    /// encoding name = "ieee-754", encoding code = 0x72
    /// category = fixed, width = 4
    /// label = "IEEE 754-2008 binary32"
    Float(OrderedFloat<f32>),

    /// 64-bit floating point number (IEEE 754-2008 binary64).
    ///
    /// encoding name = "ieee-754", encoding code = 0x82
    /// category = fixed, width = 8
    /// label = "IEEE 754-2008 binary64"
    Double(OrderedFloat<f64>),

    /// 32-bit decimal number (IEEE 754-2008 decimal32).
    ///
    /// encoding name = "ieee-754", encoding code = 0x74
    /// category = fixed, width = 4
    /// label = "IEEE 754-2008 decimal32 using the Binary Integer Decimal encoding"
    Decimal32(Dec32),

    /// 64-bit decimal number (IEEE 754-2008 decimal64).
    ///
    /// encoding name = "ieee-754", encoding code = 0x84
    /// category = fixed, width = 8
    /// label = "IEEE 754-2008 decimal64 using the Binary Integer Decimal encoding"
    Decimal64(Dec64),

    /// 128-bit decimal number (IEEE 754-2008 decimal128).
    ///
    /// encoding name = "ieee-754", encoding code = 0x94
    /// category = fixed, width = 16
    /// label = "IEEE 754-2008 decimal128 using the Binary Integer Decimal encoding"
    Decimal128(Dec128),

    /// A single Unicode character
    ///
    /// encoding name = "utf32", encoding code = 0x73
    /// category = fixed, width = 4,
    /// label = "a UTF-32BE encoded Unicode character"
    Char(char),

    /// An absolute point in time
    ///
    /// encoding name = "ms64", code = 0x83,
    /// category = fixed, width = 8
    /// label = "64-bit two’s-complement integer representing milliseconds since the unix epoch"
    Timestamp(Timestamp),

    /// A universally unique identifier as defined by RFC-4122 in section 4.1.2
    ///
    /// encoding code = 0x98,
    /// category = fixed, width = 16,
    /// label="UUID as defined in section 4.1.2 of RFC-4122"
    Uuid(Uuid),

    /// A sequence of octets.
    ///
    /// encoding name = "vbin8", encoding code = 0xa0
    /// category = variable, width = 1
    /// label = "up to 2^8 - 1 octets of binary data"
    ///
    /// encoding name = "vbin32", encoding code = 0xb0,
    /// category = variable, width = 4,
    /// label="up to 2^32 - 1 octets of binary data"
    Binary(ByteBuf),

    /// A sequence of Unicode characters.
    ///
    /// encoding name = "str8-utf8", encoding code = 0xa1,
    /// category = variable, width = 1
    /// label = "up to 2^8 - 1 octets worth of UTF-8 Unicode (with no byte order mark)"
    ///
    /// encoding name = "str32-utf8", encoding code = 0xb1
    /// category = variable, width = 4
    /// label="up to 2^32 - 1 octets worth of UTF-8 Unicode (with no byte order mark)"
    String(String),

    /// Symbolic values from a constrained domain.
    ///
    /// encoding name = "sym8", encoding code = 0xa3,
    /// category = variable, width = 1
    /// label="up to 2^8 - 1 seven bit ASCII characters representing a symbolic value"
    ///
    /// encoding name = "sym32", encoding code = 0xb3
    /// category = variable, width = 4
    /// label="up to 2^32 - 1 seven bit ASCII characters representing a symbolic value"
    ///
    /// Symbols are values from a constrained domain.
    /// Although the set of possible domains is open-ended,
    /// typically the both number and size of symbols in use for any
    /// given application will be small, e.g. small enough that it is reasonable
    /// to cache all the distinct values. Symbols are encoded as ASCII characters [ASCII].
    Symbol(Symbol),

    /// A sequence of polymorphic values.
    ///
    /// encoding name = "list0", encoding code = 0x45
    /// category = fixed, width = 0,
    /// label="the empty list (i.e. the list with no elements)"
    ///
    /// encoding name = "list8", encoding code = 0xc0
    /// category = compound, width = 1
    /// label="up to 2^8 - 1 list elements with total size less than 2^8 octets
    ///
    /// encoding name = "list32", encoding code = 0xd0
    /// category = compound, width = 4
    /// label="up to 2^32 - 1 list elements with total size less than 2^32 octets"
    List(Vec<Value>),

    /// A polymorphic mapping from distinct keys to values.
    ///
    /// encoding name = "map8", encoding code = 0xc1,
    /// category = compound, width = 1
    /// label="up to 2^8 - 1 octets of encoded map data"
    ///
    /// encoding name = "map32", encoding code = 0xd1,
    /// category = compound, width = 4
    /// label="up to 2^32 - 1 octets of encoded map data
    ///
    /// Map encodings MUST contain an even number of items (i.e. an equal number of keys and values).
    /// A map in which there exist two identical key values is invalid. Unless known to be otherwise,
    /// maps MUST be considered to be ordered, that is, the order of the key-value pairs is semantically
    /// important and two maps which are different only in the order in which their key-value pairs are
    /// encoded are not equal.
    ///
    /// Note: Can only use BTreeMap as it must be considered to be ordered
    Map(BTreeMap<Value, Value>),

    /// A sequence of values of a single type.
    ///
    /// encoding name = "array8", encoding code = 0xe0
    /// category = array, width = 1,
    /// label="up to 2^8 - 1 array elements with total size less than 2^8 octets"
    ///
    /// encoding name = "array32", encoding code = 0xf0,
    /// category = array, width = 4
    /// label="up to 2^32 - 1 array elements with total size less than 2^32 octets"
    Array(Vec<Value>),
}
