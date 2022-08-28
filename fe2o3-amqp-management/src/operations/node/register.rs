
use fe2o3_amqp_types::messaging::{Message, ApplicationProperties};

use crate::{error::Result, request::MessageSerializer, operations::{OPERATION, REGISTER}};

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
pub struct RegisterRequest {
    address: String,
}

impl MessageSerializer for RegisterRequest {
    type Body = ();

    fn into_message(self) -> fe2o3_amqp_types::messaging::Message<Self::Body> {
        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, REGISTER)
                    .insert("address", self.address)
                    .build()
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
pub struct RegisterResponse {

}

impl RegisterResponse {
    const STATUS_CODE: u16 = 200;
}