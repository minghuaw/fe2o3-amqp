use fe2o3_amqp::types::Uint;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Milliseconds(Uint);