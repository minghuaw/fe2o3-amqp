//! Primitive types defined in the AMQP1.0 specification Part 1.6

use std::convert::{TryFrom, TryInto};

use ordered_float::OrderedFloat;
use serde::{de, ser};
use serde_amqp::format_code::EncodingCodes;
use serde_bytes::ByteBuf;

pub use serde_amqp::primitives::*;
pub use serde_amqp::{lazy::LazyValue, value::Value};

mod simple_value;
pub use simple_value::SimpleValue;
