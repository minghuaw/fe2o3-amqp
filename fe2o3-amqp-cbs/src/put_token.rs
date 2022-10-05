use fe2o3_amqp::types::{
    messaging::{ApplicationProperties, Message},
    primitives::Value,
};
use fe2o3_amqp_management::{
    constants::{NAME, OPERATION},
    request::MessageSerializer,
    response::MessageDeserializer,
};
use std::borrow::Cow;

use crate::constants::PUT_TOKEN;

pub struct PutTokenRequest<'a> {
    pub name: Cow<'a, str>,
    pub token: Cow<'a, str>,
}

impl<'a> PutTokenRequest<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>, token: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            token: token.into(),
        }
    }
}

impl<'a> MessageSerializer for PutTokenRequest<'a> {
    type Body = String;

    fn into_message(self) -> fe2o3_amqp::types::messaging::Message<Self::Body> {
        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, PUT_TOKEN)
                    .insert(NAME, self.name.to_string())
                    .build(),
            )
            .value(self.token.to_string())
            .build()
    }
}

pub struct PutTokenResponse {}

impl PutTokenResponse {
    pub const STATUS_CODE: u16 = 202;
}

impl MessageDeserializer<Value> for PutTokenResponse {
    type Error = fe2o3_amqp_management::error::Error;

    fn from_message(_message: Message<Value>) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}
