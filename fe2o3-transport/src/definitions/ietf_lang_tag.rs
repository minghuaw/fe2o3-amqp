use fe2o3_amqp::types::Symbol;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct IetfLanguageTag(Symbol);
