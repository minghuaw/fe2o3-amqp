use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::{SimpleValue, Value},
};

use crate::{
    constants::{DEREGISTER, LOCALES, OPERATION, TYPE},
    error::{Error, Result},
    request::Request,
    response::Response,
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

impl<'a> Request for DeregisterRequest<'a> {
    type Response = DeregisterResponse;
    type Body = ();

    fn into_message(self) -> fe2o3_amqp_types::messaging::Message<Self::Body> {
        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, DEREGISTER)
                    .insert(TYPE, self.r#type.to_string())
                    .insert(
                        LOCALES,
                        self.locales
                            .map(|s| SimpleValue::from(s.to_string()))
                            .unwrap_or(SimpleValue::Null),
                    )
                    .insert("address", self.address.to_string())
                    .build(),
            )
            .body(())
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

impl DeregisterResponse {}

impl Response for DeregisterResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Value;

    type Error = Error;
    type StatusError = Error;

    fn from_message(mut message: Message<Value>) -> Result<Self> {
        let _status_code = Self::check_status_code(&mut message)?;

        Ok(Self {})
    }
}
