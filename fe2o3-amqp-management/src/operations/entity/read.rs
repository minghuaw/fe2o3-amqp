use std::collections::BTreeMap;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Body, Message},
    primitives::Value,
};

use crate::{error::Result, request::MessageSerializer, operations::{OPERATION, READ}};

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

impl MessageSerializer for ReadRequest {
    type Body = ();

    fn into_message(self) -> Message<Self::Body> {
        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, READ)
                    .insert("name", self.name)
                    .insert("identity", self.identity)
                    .build()
            )
            .value(())
            .build()
    }
}

pub struct ReadResponse {
    entity_attributes: BTreeMap<String, Value>,
}

impl ReadResponse {
    const STATUS_CODE: u16 = 200;
}
