use std::collections::BTreeMap;

use fe2o3_amqp_types::primitives::Value;

use crate::error::Result;

pub trait GetOperations {
    fn get_operations(&self, req: GetOperationsRequest) -> Result<GetOperationsResponse>;
}

/// GET-OPERATIONS
/// 
/// Retrieve the list of Management Operations (and the arguments they take) which can be performed
/// via this Management Node.
/// 
/// Body
/// 
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct GetOperationsRequest {
    entity_type: Option<String>
}

pub struct GetOperationsResponse {
    map: BTreeMap<Value, BTreeMap<String, Vec<String>>>
}