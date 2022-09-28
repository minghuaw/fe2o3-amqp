use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{AmqpValue, ApplicationProperties, Body, Message},
    primitives::OrderedMap,
};

use crate::{
    error::{Error, Result},
    constants::{GET_OPERATIONS, OPERATION},
    request::MessageSerializer,
    response::MessageDeserializer,
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
pub struct GetOperationsRequest<'a> {
    pub entity_type: Option<Cow<'a, str>>,
}

impl<'a> GetOperationsRequest<'a> {
    pub fn new(entity_type: impl Into<Option<Cow<'a, str>>>) -> Self {
        Self { entity_type: entity_type.into() }
    }
}

impl<'a> MessageSerializer for GetOperationsRequest<'a> {
    type Body = ();

    fn into_message(self) -> Message<Self::Body> {
        let mut builder = ApplicationProperties::builder();
        builder = builder.insert(OPERATION, GET_OPERATIONS);
        if let Some(entity_type) = self.entity_type {
            builder = builder.insert("entityType", entity_type.to_string());
        }
        Message::builder()
            .application_properties(builder.build())
            .value(())
            .build()
    }
}

/// If the request was successful then the statusCode MUST be 200 (OK) and the body of the message
/// MUST consist of an amqp-value section containing a map. The keys in the map MUST be the set of
/// Manageable Entity Types for which the list of Management Operations is being provided. For any
/// given key, the value MUST itself be a map, where each key is the string name of a Management
/// Operation that can be performed against this Manageable Entity Type via this Management Node,
/// and the value for a given key is a list of strings giving the names of the arguments (passed via
/// the application- properties of a request message) which the operation defines. For any given
/// Manageable Entity Type, the set of operations returned MUST include every operation supported by
/// Manageable Entity Types that it extends, either directly or indirectly.
pub struct GetOperationsResponse {
    pub operations: OrderedMap<String, OrderedMap<String, Vec<String>>>,
}

impl GetOperationsResponse {
    pub const STATUS_CODE: u16 = 200;
}

impl MessageDeserializer<OrderedMap<String, OrderedMap<String, Vec<String>>>>
    for GetOperationsResponse
{
    type Error = Error;

    fn from_message(
        message: Message<OrderedMap<String, OrderedMap<String, Vec<String>>>>,
    ) -> Result<Self> {
        match message.body {
            Body::Value(AmqpValue(operations)) => Ok(Self { operations }),
            _ => Err(Error::DecodeError),
        }
    }
}
