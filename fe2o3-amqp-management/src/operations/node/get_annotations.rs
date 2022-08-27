use std::collections::BTreeMap;

use fe2o3_amqp_types::primitives::Value;

pub struct GetAnnotationsRequestProperties {
    entity_type: Option<String>,
}

pub struct GetAnnotationsResponse {
    map: BTreeMap<Value, Vec<String>>,
}

impl GetAnnotationsResponse {
    const STATUS_CODE: u16 = 200;
}
