use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};

use fe2o3_amqp::{types::Symbol, value::Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct Fields(BTreeMap<Symbol, Value>); // TODO: change val type to Value