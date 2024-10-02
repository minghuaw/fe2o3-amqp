// Public constants that are also used in derive macros
#[doc(hidden)]
pub const DESCRIBED_BASIC: &str = "AMQP1.0_DESCRIBED_BASIC";
#[doc(hidden)]
pub const DESCRIBED_LIST: &str = "AMQP1.0_DESCRIBED_LIST";
#[doc(hidden)]
pub const DESCRIBED_MAP: &str = "AMQP1.0_DESCRIBED_MAP";
#[doc(hidden)]
pub const DESCRIPTOR: &str = "AMQP1.0_DESCRIPTOR";

#[doc(hidden)]
pub const UNTAGGED_ENUM: &str = "FE2O3_AMQP_UNTAGGED";

/// Use [`VALUE`] as the name if an enum needs to peek the format code before performing
/// deserialization
pub const VALUE: &str = "AMQP1.0_VALUE";

// These constants should only be public on the crate level
// to avoid accidental misuse
pub(crate) const ARRAY: &str = "AMQP1.0_ARRAY";
pub(crate) const DECIMAL32: &str = "AMQP1.0_DECIMAL32";
pub(crate) const DECIMAL64: &str = "AMQP1.0_DECIMAL64";
pub(crate) const DECIMAL128: &str = "AMQP1.0_DECIMAL128";
pub(crate) const SYMBOL: &str = "AMQP1.0_SYMBOL";
pub(crate) const SYMBOL_REF: &str = "AMQP1.0_SYMBOL_REF";
pub(crate) const TIMESTAMP: &str = "AMQP1.0_TIMESTAMP";
pub(crate) const UUID: &str = "AMQP1.0_UUID";

// This is not a type defined in the standard
pub(crate) const TRANSPARENT_VEC: &str = "__TRANSPARENT_VEC";
pub(crate) const LAZY_VALUE: &str = "__LAZY_VALUE";
