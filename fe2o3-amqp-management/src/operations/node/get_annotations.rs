use std::collections::BTreeMap;

use fe2o3_amqp_types::{primitives::Value, messaging::{Message, ApplicationProperties}};

use crate::{error::Result, request::MessageSerializer, operations::{OPERATION, GET_ANNOTATIONS}};

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

impl MessageSerializer for GetAnnotationsRequest {
    type Body = ();

    fn into_message(self) -> Message<Self::Body> {
        let mut builder  = ApplicationProperties::builder();
        builder = builder.insert(OPERATION, GET_ANNOTATIONS);
        if let Some(entity_type) = self.entity_type {
            builder = builder.insert("entityType", entity_type);
        }
        Message::builder()
            .application_properties(builder.build())
            .value(())
            .build()
    }
}

pub struct GetAnnotationsResponse {
    map: BTreeMap<Value, Vec<String>>,
}

impl GetAnnotationsResponse {
    const STATUS_CODE: u16 = 200;
}
