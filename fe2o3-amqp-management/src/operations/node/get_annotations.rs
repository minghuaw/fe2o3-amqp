use std::collections::BTreeMap;

use fe2o3_amqp_types::primitives::Value;

use crate::error::Result;

pub trait GetAnnotations {
    fn get_annotations(&self, req: GetAnnotationsRequest) -> Result<GetAnnotationsResponse>;
}

/// GET-ANNOTATIONS
/// 
/// Body:
/// 
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct GetAnnotationsRequest {
    entity_type: Option<String>,
}

pub struct GetAnnotationsResponse {
    map: BTreeMap<Value, Vec<String>>,
}

impl GetAnnotationsResponse {
    const STATUS_CODE: u16 = 200;
}
