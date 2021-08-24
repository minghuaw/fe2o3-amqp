use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};

use fe2o3_amqp::types::Symbol;

#[derive(Debug, Serialize, Deserialize)]
pub struct Fields(BTreeMap<Symbol, Vec<u8>>); // TODO: change val type to Value