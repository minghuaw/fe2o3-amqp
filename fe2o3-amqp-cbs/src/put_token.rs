use fe2o3_amqp::types::{
    messaging::{ApplicationProperties, Message},
    primitives::{SimpleValue, Timestamp, Value},
};
use fe2o3_amqp_management::{
    constants::{LOCALES, NAME, OPERATION, TYPE},
    request::Request,
    response::Response,
};
use std::borrow::Cow;

use crate::constants::{EXPIRATION, PUT_TOKEN};

/// # Panic
///
/// Conversion from [`PutTokenRequest`] to [`Message`] will panic if `OffsetDateTime` represented in
/// unix time but with a precision of milliseconds exceeds [`i64::MIN`] or [`i64::MAX`].
pub struct PutTokenRequest<'a> {
    pub name: Cow<'a, str>,
    pub token: Cow<'a, str>,
    pub expiration: Option<Timestamp>,
    pub r#type: Cow<'a, str>,
    pub locales: Option<Cow<'a, str>>,
}

impl<'a> PutTokenRequest<'a> {
    pub fn new(
        name: impl Into<Cow<'a, str>>,
        token: impl Into<Cow<'a, str>>,
        expiration: impl Into<Option<Timestamp>>,
        r#type: impl Into<Cow<'a, str>>,
        locales: impl Into<Option<Cow<'a, str>>>,
    ) -> Self {
        Self {
            name: name.into(),
            token: token.into(),
            expiration: expiration.into(),
            r#type: r#type.into(),
            locales: locales.into(),
        }
    }
}

impl<'a> Request for PutTokenRequest<'a> {
    type Response = PutTokenResponse;
    type Body = String;

    fn into_message(self) -> fe2o3_amqp::types::messaging::Message<Self::Body> {
        let expiration = match self.expiration {
            Some(timestamp) => SimpleValue::Timestamp(timestamp),
            None => SimpleValue::Null,
        };
        let props = ApplicationProperties::builder()
            .insert(TYPE, SimpleValue::String(self.r#type.into()))
            .insert(
                LOCALES,
                self.locales
                    .map(|s| SimpleValue::String(s.into()))
                    .unwrap_or(SimpleValue::Null),
            )
            .insert(OPERATION, PUT_TOKEN)
            .insert(NAME, self.name.to_string())
            .insert(EXPIRATION, expiration)
            .build();
        Message::builder()
            .application_properties(props)
            .body(self.token.to_string())
            .build()
    }
}

pub struct PutTokenResponse {}

impl PutTokenResponse {}

impl Response for PutTokenResponse {
    const STATUS_CODE: u16 = 202;

    type Body = Value;

    type Error = fe2o3_amqp_management::error::Error;
    type StatusError = fe2o3_amqp_management::error::Error;

    fn from_message(mut message: Message<Value>) -> Result<Self, Self::Error> {
        let _status_code = Self::check_status_code(&mut message)?;
        Ok(Self {})
    }
}
