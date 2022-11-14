use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::{OrderedMap, SimpleValue},
};

use crate::{
    constants::{GET_ATTRIBUTES, OPERATION, TYPE, LOCALES},
    error::{Error, Result}, request::Request, response::Response,
};

use super::get::GetRequest;

pub trait GetAttributes {
    fn get_attributes(&self, req: GetAttributesRequest) -> Result<GetAttributesResponse>;
}

/// GET-ATTRIBUTES
///
/// Body:
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct GetAttributesRequest<'a> {
    inner: GetRequest<'a>
}

impl<'a> GetAttributesRequest<'a> {
    pub fn new(
        entity_type: impl Into<Option<Cow<'a, str>>>,
        r#type: impl Into<Cow<'a, str>>,
        locales: Option<impl Into<Cow<'a, str>>>,
    ) -> Self {
        Self {
            inner: GetRequest::new(entity_type, r#type, locales)
        }
    }
}

impl<'a> Request for GetAttributesRequest<'a> {
    type Response = GetAttributesResponse;
    type Body = ();

    fn into_message(self) -> Message<Self::Body> {
        let mut application_properties = self.inner.into_application_properties();
        application_properties.insert(OPERATION.into(), GET_ATTRIBUTES.into());

        Message::builder()
            .application_properties(application_properties)
            .body(())
            .build()
    }
}

/// If the request was successful then the statusCode MUST be 200 (OK) and the body of the message
/// MUST contain a map. The keys in the map MUST be the set of Manageable Entity Types for which
/// attribute names are being provided. For any given key, the value MUST be a list of strings
/// representing the attribute names that this Manageable Entity Type possesses. It should be noted
/// that for each entry in the map, the attribute names returned MUST be only those defined by the
/// associated Manageable Entity Type rather than those that are defined by other Manageable Entity
/// Types that extend it. For any given Manageable Entity Type, the set of attribute names returned
/// MUST include every attribute name defined by Manageable Entity Types that it extends, either
/// directly or indirectly.
pub struct GetAttributesResponse {
    pub attributes: OrderedMap<String, Vec<String>>,
}

impl GetAttributesResponse {
}

impl Response for GetAttributesResponse {
    const STATUS_CODE: u16 = 200;
    
    type Body = Option<OrderedMap<String, Vec<String>>>;

    type Error = Error;
    type StatusError = Error;

    fn from_message(mut message: Message<Option<OrderedMap<String, Vec<String>>>>) -> Result<Self> {
        let _status_code = Self::check_status_code(&mut message)?;

        match message.body {
            Some(attributes) => Ok(Self { attributes }),
            None => Ok(Self {
                attributes: OrderedMap::with_capacity(0),
            }),
        }
    }
}
