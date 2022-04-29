#![deny(missing_docs, missing_debug_implementations)]

//! Implements AMQP1.0 data types as defined in the [specification](http://docs.oasis-open.org/amqp/core/v1.0/os/amqp-core-overview-v1.0-os.html).

#[cfg(feature = "primitive")]
pub mod primitives;

#[cfg(feature = "transport")]
pub mod definitions;

#[cfg(feature = "messaging")]
pub mod messaging;

#[cfg(all(feature = "transport", feature = "messaging"))]
pub mod performatives;

#[cfg(feature = "security")]
pub mod sasl;

#[cfg(feature = "transport")]
pub mod states;

#[cfg(feature = "transaction")]
pub mod transaction;
