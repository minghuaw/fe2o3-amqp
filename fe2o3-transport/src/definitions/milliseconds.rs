use fe2o3_amqp::types::Uint;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Milliseconds(Uint);
