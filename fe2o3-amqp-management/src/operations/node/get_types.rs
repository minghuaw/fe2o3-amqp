use fe2o3_amqp_types::{
    messaging::{AmqpValue, ApplicationProperties, Body, Message},
    primitives::OrderedMap,
};

use crate::{
    error::{Error, Result},
    operations::{GET_TYPES, OPERATION},
    request::MessageSerializer,
    response::MessageDeserializer,
};

pub trait GetTypes {
    fn get_types(&self, req: GetTypesRequest) -> Result<GetTypesResponse>;
}

/// GET-TYPES
///
/// Body:
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct GetTypesRequest {
    entity_type: Option<String>,
}

impl MessageSerializer for GetTypesRequest {
    type Body = ();

    fn into_message(self) -> Message<Self::Body> {
        let mut builder = ApplicationProperties::builder();
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
    pub types: OrderedMap<String, Vec<String>>,
}

impl GetTypesResponse {
    pub const STATUS_CODE: u16 = 200;
}

impl MessageDeserializer<OrderedMap<String, Vec<String>>> for GetTypesResponse {
    type Error = Error;

    fn from_message(message: Message<OrderedMap<String, Vec<String>>>) -> Result<Self> {
        match message.body {
            Body::Value(AmqpValue(types)) => Ok(Self { types }),
            _ => Err(Error::DecodeError),
        }
    }
}
