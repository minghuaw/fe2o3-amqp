use fe2o3_amqp::types::{
    messaging::{ApplicationProperties, Message},
    primitives::{Value, Timestamp, SimpleValue},
};
use fe2o3_amqp_management::{
    constants::{NAME, OPERATION},
    request::MessageSerializer,
    response::MessageDeserializer,
};
use std::{borrow::Cow};

use crate::{constants::{PUT_TOKEN, EXPIRATION}, token::CbsToken};

/// # Panic
/// 
/// Conversion from [`PutTokenRequest`] to [`Message`] will panic if `OffsetDateTime` represented in
/// unix time but with a precision of milliseconds exceeds [`i64::MIN`] or [`i64::MAX`].
pub struct PutTokenRequest<'a> {
    pub name: Cow<'a, str>,
    pub token: Cow<'a, str>,
    pub expiration: Option<Timestamp>,
}

impl<'a> PutTokenRequest<'a> {
    pub fn new(
        name: impl Into<Cow<'a, str>>,
        token: impl Into<Cow<'a, str>>,
        expiration: impl Into<Option<Timestamp>>,
    ) -> Self {
        Self {
            name: name.into(),
            token: token.into(),
            expiration: expiration.into(),
        }
    }
}

impl<'a> From<CbsToken<'a>> for PutTokenRequest<'a> {
    fn from(token: CbsToken<'a>) -> Self {
        Self {
            name: token.name,
            token: token.token_value,
            expiration: token.expires_at_utc,
        }
    }
}

impl<'a> MessageSerializer for PutTokenRequest<'a> {
    type Body = String;

    fn into_message(self) -> fe2o3_amqp::types::messaging::Message<Self::Body> {
        let expiration = match self.expiration {
            Some(timestamp) => SimpleValue::Timestamp(timestamp),
            None => SimpleValue::Null,
        };
        let props = ApplicationProperties::builder()
            .insert(OPERATION, PUT_TOKEN)
            .insert(NAME, self.name.to_string())
            .insert(EXPIRATION, expiration)
            .build();
        Message::builder()
            .application_properties(
                props
            )
            .body(self.token.to_string())
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
