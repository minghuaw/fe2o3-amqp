use std::collections::BTreeMap;

use fe2o3_amqp_types::primitives::Value;

pub struct GetOperationsRequest {
    entity_type: Option<String>
}

pub struct GetOperationsResponse {
    map: BTreeMap<Value, BTreeMap<String, Vec<String>>>
}