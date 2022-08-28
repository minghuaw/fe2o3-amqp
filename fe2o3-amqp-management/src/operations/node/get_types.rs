use std::collections::BTreeMap;

use fe2o3_amqp_types::primitives::Value;

use crate::{error::Result, request::MessageSerializer};

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

impl MessageSerializer for GetTypesRequest {
    type Body = ();

    fn into_message(self) -> fe2o3_amqp_types::messaging::Message<Self::Body> {
        todo!()
    }
}

pub struct GetTypesResponse {
    map: BTreeMap<Value, Vec<String>>,
}

impl GetTypesResponse {
    const STATUS_CODE: u16 = 200;
}