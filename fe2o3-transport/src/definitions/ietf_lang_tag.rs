use fe2o3_amqp::types::Symbol;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct IetfLanguageTag(Symbol);
