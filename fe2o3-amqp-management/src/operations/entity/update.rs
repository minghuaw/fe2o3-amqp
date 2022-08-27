use std::collections::BTreeMap;

use fe2o3_amqp_types::{primitives::Value, messaging::{Message, ApplicationProperties, AmqpValue, Body}};

use crate::{error::Result, request::IntoMessageFields};

pub trait Update {
    fn update(&mut self, arg: UpdateRequest) -> Result<UpdateResponse>;
}

/// Update a Manageable Entity.
/// 
/// # Body: 
/// 
/// The body MUST consist of an amqp-value section containing a map. The map consists of key-value
/// pairs where the key represents the name of an attribute of the entity and the value represents
/// the initial value it SHOULD take. The absence of an attribute name implies that the entity
/// should retain its existing value.
/// 
/// If the map contains a key-value pair where the value is null then the updated entity should have
/// no value for that attribute, removing any previous value.
/// 
/// In the case where the supplied map contains multiple attributes, then the update MUST be treated
/// as a single, atomic operation so if any of the changes cannot be applied, none of the attributes
/// in the map should be updated and this MUST result in a failure response.
/// 
/// Where the type of the attribute value provided is not as required, type conversion as per the
/// rules in 3.3.1.1 MUST be provided.
pub struct UpdateRequest {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    pub name: String,

    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    pub identity: String,

    pub body: BTreeMap<String, Value>,
}

impl<T> IntoMessageFields<T> for UpdateRequest {
    type Body = BTreeMap<String, Value>;

    fn into_message_fields(self, mut message: Message<T>) -> Message<Self::Body> {
        let application_properties = message.application_properties.get_or_insert(ApplicationProperties::default());
        application_properties.insert(String::from("name"), self.name.into());
        application_properties.insert(String::from("identity"), self.identity.into());

        let body = Body::Value(AmqpValue(self.body));
        Message {
            header: message.header,
            delivery_annotations: message.delivery_annotations,
            message_annotations: message.message_annotations,
            properties: message.properties,
            application_properties: message.application_properties,
            body,
            footer: message.footer,
        }
    }
}

/// If the request was successful then the statusCode MUST contain 200 (OK) and the body of the
/// message MUST consists of an amqp-value section containing a map of the actual attributes of the
/// entity updated. These MAY differ from those requested. 
/// 
/// A map containing attributes that are not
/// applicable for the entity being created, or an invalid value for a given attribute (excepting
/// type conversion as above), MUST result in a failure response with a statusCode of 400 (Bad
/// Request).
pub struct UpdateResponse {
    entity_attributes: BTreeMap<String, Value>,
}

impl UpdateResponse {
    const STATUS_CODE: u16 = 200;
}