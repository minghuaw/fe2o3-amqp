#![deny(missing_docs, missing_debug_implementations)]

//! Implements AMQP1.0 data types as defined in the [specification](http://docs.oasis-open.org/amqp/core/v1.0/os/amqp-core-overview-v1.0-os.html).

pub mod definitions;
pub mod messaging;
pub mod performatives;
pub mod primitives;
pub mod sasl;
pub mod states;
pub mod transaction;
