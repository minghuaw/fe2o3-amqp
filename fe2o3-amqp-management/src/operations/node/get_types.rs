use std::collections::BTreeMap;

use fe2o3_amqp_types::{primitives::Value, messaging::{ApplicationProperties, Message}};

use crate::{error::Result, request::MessageSerializer, operations::{OPERATION, GET_TYPES}};

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

    fn into_message(self) -> Message<Self::Body> {
        let mut builder  = ApplicationProperties::builder();
        builder = builder.insert(OPERATION, GET_TYPES);
        if let Some(entity_type) = self.entity_type {
            builder = builder.insert("entityType", entity_type);
        }
        Message::builder()
            .application_properties(builder.build())
            .value(())
            .build()
    }
}

pub struct GetTypesResponse {
    map: BTreeMap<Value, Vec<String>>,
}

impl GetTypesResponse {
    const STATUS_CODE: u16 = 200;
}