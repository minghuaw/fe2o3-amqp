use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::Value,
};

use crate::{
    error::{Error, Result},
    constants::{OPERATION, REGISTER},
    request::MessageSerializer,
    response::MessageDeserializer,
};

pub trait Register {
    fn register(&mut self, req: RegisterRequest) -> Result<RegisterResponse>;
}

/// REGISTER
///
/// Register a Management Node.
///
/// Body
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct RegisterRequest<'a> {
    address: Cow<'a, str>,
}

impl<'a> MessageSerializer for RegisterRequest<'a> {
    type Body = ();

    fn into_message(self) -> fe2o3_amqp_types::messaging::Message<Self::Body> {
        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, REGISTER)
                    .insert("address", self.address.to_string())
                    .build(),
            )
            .value(())
            .build()
    }
}

/// No information is carried in the message body therefore any message body is valid and MUST be
/// ignored.
///
/// If the request was successful then the statusCode MUST be 200 (OK). Upon a successful
/// registration, the address of the registered Management Node will be present in the list of known
/// Management Nodes returned by subsequent GET-MGMT-NODES operations.
pub struct RegisterResponse {}

impl RegisterResponse {
    pub const STATUS_CODE: u16 = 200;
}

impl MessageDeserializer<Value> for RegisterResponse {
    type Error = Error;

    fn from_message(_message: Message<Value>) -> Result<Self> {
        Ok(Self {})
    }
}
