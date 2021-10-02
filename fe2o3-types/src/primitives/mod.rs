use std::convert::{TryFrom, TryInto};

use serde_amqp::{format_code::EncodingCodes};
use ordered_float::OrderedFloat;
use serde::{de, ser};
use serde_bytes::ByteBuf;

pub use serde_amqp::primitives::*;
pub use serde_amqp::value::Value;

mod simple_value;
pub use simple_value::SimpleValue;
