use fe2o3_amqp::types::{
    messaging::{ApplicationProperties, Message},
    primitives::{SimpleValue, Timestamp, Value},
};
use fe2o3_amqp_management::{
    constants::{NAME},
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
    pub manageable_entity_type: Cow<'a, str>,
    pub locales: Option<Cow<'a, str>>,
}

impl<'a> PutTokenRequest<'a> {
    pub fn new(
        name: impl Into<Cow<'a, str>>,
        token: impl Into<Cow<'a, str>>,
        expiration: impl Into<Option<Timestamp>>,
        manageable_entity_type: impl Into<Cow<'a, str>>,
        locales: impl Into<Option<Cow<'a, str>>>,
    ) -> Self {
        Self {
            name: name.into(),
            token: token.into(),
            expiration: expiration.into(),
            manageable_entity_type: manageable_entity_type.into(),
            locales: locales.into(),
        }
    }
}

impl<'a> Request for PutTokenRequest<'a> {
    const OPERATION: &'static str = PUT_TOKEN;

    type Response = PutTokenResponse;

    type Body = String;

    fn manageable_entity_type(&mut self) -> Option<String> {
        Some(self.manageable_entity_type.to_string())
    }

    fn locales(&mut self) -> Option<String> {
        self.locales.as_ref().map(|x| x.to_string())
    }

    fn encode_application_properties(&mut self) -> Option<ApplicationProperties> {
        let expiration = match self.expiration.take() {
            Some(timestamp) => SimpleValue::Timestamp(timestamp),
            None => SimpleValue::Null,
        };
        Some(
            ApplicationProperties::builder()
                .insert(NAME, self.name.to_string())
                .insert(EXPIRATION, expiration)
                .build(),
        )
    }

    fn encode_body(self) -> Self::Body {
        self.token.into()
    }
}

pub struct PutTokenResponse {}

impl PutTokenResponse {}

impl Response for PutTokenResponse {
    const STATUS_CODE: u16 = 202;

    type Body = Value;

    type Error = fe2o3_amqp_management::error::Error;

    fn decode_message(_message: Message<Self::Body>) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}
