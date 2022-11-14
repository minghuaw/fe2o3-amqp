use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::Value,
};

use crate::{constants::DEREGISTER, error::Error, request::Request, response::Response};

pub trait Deregister {
    fn deregister(&mut self, req: DeregisterRequest) -> Result<DeregisterResponse, Error>;
}

/// DEREGISTER
///
/// Delete the registration of a Management Node.
///
/// # Body
///
/// The body of the message MUST be empty.
pub struct DeregisterRequest<'a> {
    pub address: Cow<'a, str>,

    /// Entity type
    pub r#type: Cow<'a, str>,

    /// locales
    pub locales: Option<Cow<'a, str>>,
}

impl<'a> DeregisterRequest<'a> {
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

impl Request for DeregisterRequest<'_> {
    const OPERATION: &'static str = DEREGISTER;

    type Response = DeregisterResponse;
    type Body = ();

    fn manageable_entity_type(&mut self) -> Option<String> {
        Some(self.r#type.to_string())
    }

    fn locales(&mut self) -> Option<String> {
        self.locales.as_ref().map(|x| x.to_string())
    }

    fn encode_application_properties(&mut self) -> Option<ApplicationProperties> {
        Some(
            ApplicationProperties::builder()
                .insert("address", self.address.to_string())
                .build(),
        )
    }

    fn encode_body(self) -> Self::Body {
        ()
    }
}

/// No information is carried in the message body therefore any message body is valid and MUST be
/// ignored.
///
/// If the request was successful then the statusCode MUST be 200 (OK). Upon a successful
/// deregistration, the address of the unregistered Management Node will not be present in the list
/// of known Management Nodes returned by subsequent GET-MGMT-NODES operations.
pub struct DeregisterResponse {}

impl DeregisterResponse {}

impl Response for DeregisterResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Value;

    type Error = Error;

    fn decode_message(_message: Message<Self::Body>) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}
