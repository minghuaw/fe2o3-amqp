use std::collections::BTreeMap;

use fe2o3_amqp_types::primitives::Value;

use crate::error::Result;

pub trait GetTypes {
    fn get_types(&self, req: GetTypesRequest) -> Result<GetTypesResponse>;
}

/// GET-TYPES
/// 
/// Body:
/// 
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct GetTypesRequest {
    entity_type: Option<String>
}

pub struct GetTypesResponse {
    map: BTreeMap<Value, Vec<String>>,
}

impl GetTypesResponse {
    const STATUS_CODE: u16 = 200;
}