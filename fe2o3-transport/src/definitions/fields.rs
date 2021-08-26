use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use fe2o3_amqp::{types::Symbol, value::Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct Fields(BTreeMap<Symbol, Value>);
