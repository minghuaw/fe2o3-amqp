use std::collections::BTreeMap;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::Value,
};

use crate::{
    error::Result,
    operations::{GET_OPERATIONS, OPERATION},
    request::MessageSerializer,
};

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
    entity_type: Option<String>,
}

impl MessageSerializer for GetOperationsRequest {
    type Body = ();

    fn into_message(self) -> Message<Self::Body> {
        let mut builder = ApplicationProperties::builder();
        builder = builder.insert(OPERATION, GET_OPERATIONS);
        if let Some(entity_type) = self.entity_type {
            builder = builder.insert("entityType", entity_type);
        }
        Message::builder()
            .application_properties(builder.build())
            .value(())
            .build()
    }
}

pub struct GetOperationsResponse {
    map: BTreeMap<Value, BTreeMap<String, Vec<String>>>,
}
