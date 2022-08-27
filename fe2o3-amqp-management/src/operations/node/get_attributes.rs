use std::collections::BTreeMap;

use fe2o3_amqp_types::primitives::Value;

pub struct GetAttributesRequestProperties {
    entity_type: Option<String>
}

pub struct GetAttributesResponse {
    map: BTreeMap<Value, Vec<String>>
}

impl GetAttributesResponse {
    const STATUS_CODE: u16 = 200;
}