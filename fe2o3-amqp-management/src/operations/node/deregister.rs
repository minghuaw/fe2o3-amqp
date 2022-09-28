use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::Value,
};

use crate::{
    error::{Error, Result},
    constants::{DEREGISTER, OPERATION},
    request::MessageSerializer,
    response::MessageDeserializer,
};

pub trait Deregister {
    fn deregister(&mut self, req: DeregisterRequest) -> Result<DeregisterResponse>;
}

/// DEREGISTER
///
/// Delete the registration of a Management Node.
///
/// # Body
///
/// The body of the message MUST be empty.
pub struct DeregisterRequest {
    address: String,
}

impl MessageSerializer for DeregisterRequest {
    type Body = ();

    fn into_message(self) -> fe2o3_amqp_types::messaging::Message<Self::Body> {
        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, DEREGISTER)
                    .insert("address", self.address)
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
/// deregistration, the address of the unregistered Management Node will not be present in the list
/// of known Management Nodes returned by subsequent GET-MGMT-NODES operations.
pub struct DeregisterResponse {}

impl DeregisterResponse {
    pub const STATUS_CODE: u16 = 200;
}

impl MessageDeserializer<Value> for DeregisterResponse {
    type Error = Error;

    fn from_message(_message: Message<Value>) -> Result<Self> {
        Ok(Self {})
    }
}
