use serde::{Deserialize, Serialize};

pub mod definitions;
pub mod performatives;
pub mod framing;
pub mod messaging;

#[derive(Debug, Deserialize, Serialize)]
pub struct Source {}

#[derive(Debug, Deserialize, Serialize)]
pub struct Target {}
