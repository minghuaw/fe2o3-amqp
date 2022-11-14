use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::{Value, SimpleValue},
};

use crate::{
    constants::{OPERATION, REGISTER, TYPE, LOCALES},
    error::{Error, Result}, request::Request, response::Response,
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
    type Body = ();

    fn into_message(self) -> fe2o3_amqp_types::messaging::Message<Self::Body> {
        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, REGISTER)
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
/// registration, the address of the registered Management Node will be present in the list of known
/// Management Nodes returned by subsequent GET-MGMT-NODES operations.
pub struct RegisterResponse { }

impl RegisterResponse {
}

impl Response for RegisterResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Value;

    type Error = Error;
    type StatusError = Error;

    fn from_message(_message: Message<Value>) -> Result<Self> {


        Ok(Self {})
    }
}
