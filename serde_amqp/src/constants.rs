// Public constants that are also used in derive macros
#[doc(hidden)]
pub const DESCRIBED_BASIC: &str = "AMQP1.0_DESCRIBED_BASIC";
#[doc(hidden)]
pub const DESCRIBED_LIST: &str = "AMQP1.0_DESCRIBED_LIST";
#[doc(hidden)]
pub const DESCRIBED_MAP: &str = "AMQP1.0_DESCRIBED_MAP";
#[doc(hidden)]
pub const DESCRIPTOR: &str = "AMQP1.0_DESCRIPTOR";

// These constants should only be public on the crate level
// to avoid accidental misuse
pub(crate) const VALUE: &str = "AMQP1.0_VALUE";
pub(crate) const ARRAY: &str = "AMQP1.0_ARRAY";
pub(crate) const DECIMAL32: &str = "AMQP1.0_DECIMAL32";
pub(crate) const DECIMAL64: &str = "AMQP1.0_DECIMAL64";
pub(crate) const DECIMAL128: &str = "AMQP1.0_DECIMAL128";
pub(crate) const SYMBOL: &str = "AMQP1.0_SYMBOL";
pub(crate) const TIMESTAMP: &str = "AMQP1.0_TIMESTAMP";
pub(crate) const UUID: &str = "AMQP1.0_UUID";
