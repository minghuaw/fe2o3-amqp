use std::convert::{TryFrom, TryInto};

use fe2o3_amqp::{constants::VALUE, format_code::EncodingCodes};
use ordered_float::OrderedFloat;
use serde::{de, ser};
use serde_bytes::ByteBuf;

pub use fe2o3_amqp::primitives::*;
pub use fe2o3_amqp::value::Value;

mod simple_value;
pub use simple_value::SimpleValue;
