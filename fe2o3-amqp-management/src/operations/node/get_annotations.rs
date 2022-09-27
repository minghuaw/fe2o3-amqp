use fe2o3_amqp_types::{
    messaging::{AmqpValue, ApplicationProperties, Body, Message},
    primitives::OrderedMap,
};

use crate::{
    error::{Error, Result},
    operations::{GET_ANNOTATIONS, OPERATION},
    request::MessageSerializer,
    response::MessageDeserializer,
};

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
        let mut builder = ApplicationProperties::builder();
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
    pub annotations: OrderedMap<String, Vec<String>>,
}

impl GetAnnotationsResponse {
    pub const STATUS_CODE: u16 = 200;
}

impl MessageDeserializer<OrderedMap<String, Vec<String>>> for GetAnnotationsResponse {
    type Error = Error;

    fn from_message(message: Message<OrderedMap<String, Vec<String>>>) -> Result<Self> {
        match message.body {
            Body::Value(AmqpValue(annotations)) => Ok(Self { annotations }),
            _ => Err(Error::DecodeError),
        }
    }
}
