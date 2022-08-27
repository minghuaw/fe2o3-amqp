use std::collections::BTreeMap;

use fe2o3_amqp_types::{primitives::{Value}, messaging::{Message, ApplicationProperties}};

use crate::{error::Result, request::IntoMessageFields};

pub trait Read {
    fn read(&mut self, arg: ReadRequest) -> Result<ReadResponse>;
}

/// Retrieve the attributes of a Manageable Entity.
/// 
/// Body: No information is carried in the message body therefore any message body is valid and MUST
/// be ignored
pub struct ReadRequest {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    pub name: String,

    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    pub identity: String,
}

impl<T> IntoMessageFields<T> for ReadRequest {
    type Body = T;

    fn into_message_fields(self, mut message: Message<T>) -> Message<Self::Body> {
        let application_properties = message.application_properties.get_or_insert(ApplicationProperties::default());
        application_properties.insert(String::from("name"), self.name.into());
        application_properties.insert(String::from("identity"), self.identity.into());
        message
    }
}

pub struct ReadResponse {
    entity_attributes: BTreeMap<String, Value>,
}

impl ReadResponse {
    const STATUS_CODE: u16 = 200;
}