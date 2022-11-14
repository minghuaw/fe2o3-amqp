use std::borrow::Cow;

use fe2o3_amqp_types::{messaging::Message, primitives::OrderedMap};

use crate::{
    constants::{GET_TYPES, OPERATION},
    error::{Error, Result},
    request::Request,
    response::Response,
};

use super::get::GetRequest;

pub trait GetTypes {
    fn get_types(&self, req: GetTypesRequest) -> Result<GetTypesResponse>;
}

/// GET-TYPES
///
/// Body:
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct GetTypesRequest<'a> {
    inner: GetRequest<'a>,
}

impl<'a> GetTypesRequest<'a> {
    pub fn new(
        entity_type: impl Into<Option<Cow<'a, str>>>,
        r#type: impl Into<Cow<'a, str>>,
        locales: Option<impl Into<Cow<'a, str>>>,
    ) -> Self {
        Self {
            inner: GetRequest::new(entity_type, r#type, locales),
        }
    }
}

impl<'a> Request for GetTypesRequest<'a> {
    type Response = GetTypesResponse;
    type Body = ();

    fn into_message(self) -> Message<Self::Body> {
        let mut application_properties = self.inner.into_application_properties();
        application_properties.insert(OPERATION.into(), GET_TYPES.into());

        Message::builder()
            .application_properties(application_properties)
            .body(())
            .build()
    }
}

pub struct GetTypesResponse {
    pub types: OrderedMap<String, Vec<String>>,
}

impl GetTypesResponse {}

impl Response for GetTypesResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Option<OrderedMap<String, Vec<String>>>;

    type Error = Error;
    type StatusError = Error;

    fn from_message(mut message: Message<Option<OrderedMap<String, Vec<String>>>>) -> Result<Self> {
        let _status_code = Self::check_status_code(&mut message)?;
        match message.body {
            Some(types) => Ok(Self { types }),
            None => Ok(Self {
                types: OrderedMap::with_capacity(0),
            }),
        }
    }
}
