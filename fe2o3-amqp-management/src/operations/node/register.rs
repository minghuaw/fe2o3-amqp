use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::Value,
};

use crate::{constants::REGISTER, error::Error, request::Request, response::Response};

pub trait Register {
    fn register(&mut self, req: RegisterRequest) -> Result<RegisterResponse, Error>;
}

/// REGISTER
///
/// Register a Management Node.
///
/// Body
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct RegisterRequest<'a> {
    pub address: Cow<'a, str>,

    /// Entity type
    pub r#type: Cow<'a, str>,

    /// locales
    pub locales: Option<Cow<'a, str>>,
}

impl<'a> RegisterRequest<'a> {
    pub fn new(
        address: impl Into<Cow<'a, str>>,
        r#type: impl Into<Cow<'a, str>>,
        locales: Option<impl Into<Cow<'a, str>>>,
    ) -> Self {
        Self {
            address: address.into(),
            r#type: r#type.into(),
            locales: locales.map(|x| x.into()),
        }
    }
}

impl<'a> Request for RegisterRequest<'a> {
    const OPERATION: &'static str = REGISTER;

    type Response = RegisterResponse;
    type Body = ();

    fn manageable_entity_type(&mut self) -> Option<String> {
        Some(self.r#type.to_string())
    }

    fn locales(&mut self) -> Option<String> {
        self.locales.as_ref().map(|s| s.to_string())
    }

    fn encode_application_properties(&mut self) -> Option<ApplicationProperties> {
        Some(
            ApplicationProperties::builder()
                .insert("address", self.address.to_string())
                .build(),
        )
    }

    fn encode_body(self) -> Self::Body {}
}

/// No information is carried in the message body therefore any message body is valid and MUST be
/// ignored.
///
/// If the request was successful then the statusCode MUST be 200 (OK). Upon a successful
/// registration, the address of the registered Management Node will be present in the list of known
/// Management Nodes returned by subsequent GET-MGMT-NODES operations.
pub struct RegisterResponse {}

impl RegisterResponse {}

impl Response for RegisterResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Value;

    type Error = Error;

    fn decode_message(_message: Message<Self::Body>) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}
