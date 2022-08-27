use std::collections::BTreeMap;

use fe2o3_amqp_types::primitives::Value;

pub struct GetTypesRequestProperties {
    entity_type: Option<String>
}

pub struct GetTypesResponse {
    map: BTreeMap<Value, Vec<String>>,
}

impl GetTypesResponse {
    const STATUS_CODE: u16 = 200;
}