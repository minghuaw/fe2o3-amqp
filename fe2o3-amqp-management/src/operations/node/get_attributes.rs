use std::borrow::Cow;

use fe2o3_amqp_types::{messaging::Message, primitives::OrderedMap};

use crate::{constants::GET_ATTRIBUTES, error::Error, request::Request, response::Response};

use super::get::GetRequest;

/// A trait for handling GetAttributes request on a Manageable Node.
pub trait GetAttributes {
    /// Handles a GetAttributes request.
    fn get_attributes(&self, req: GetAttributesRequest) -> Result<GetAttributesResponse, Error>;
}

/// GET-ATTRIBUTES
///
/// Body:
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetAttributesRequest<'a> {
    inner: GetRequest<'a>,
}

impl<'a> GetAttributesRequest<'a> {
    /// Creates a new GetAttributes request.
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

impl Request for GetAttributesRequest<'_> {
    const OPERATION: &'static str = GET_ATTRIBUTES;

    type Response = GetAttributesResponse;

    type Body = ();

    fn manageable_entity_type(&mut self) -> Option<String> {
        self.inner.manageable_entity_type()
    }

    fn locales(&mut self) -> Option<String> {
        self.inner.locales()
    }

    fn encode_application_properties(
        &mut self,
    ) -> Option<fe2o3_amqp_types::messaging::ApplicationProperties> {
        self.inner.encode_application_properties()
    }

    fn encode_body(self) -> Self::Body {}
}

type GetAttributesResponseBody = OrderedMap<String, Vec<String>>;

/// If the request was successful then the statusCode MUST be 200 (OK) and the body of the message
/// MUST contain a map. The keys in the map MUST be the set of Manageable Entity Types for which
/// attribute names are being provided. For any given key, the value MUST be a list of strings
/// representing the attribute names that this Manageable Entity Type possesses. It should be noted
/// that for each entry in the map, the attribute names returned MUST be only those defined by the
/// associated Manageable Entity Type rather than those that are defined by other Manageable Entity
/// Types that extend it. For any given Manageable Entity Type, the set of attribute names returned
/// MUST include every attribute name defined by Manageable Entity Types that it extends, either
/// directly or indirectly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetAttributesResponse {
    /// The response body.
    pub body: GetAttributesResponseBody,
}

impl GetAttributesResponse {}

impl Response for GetAttributesResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Option<OrderedMap<String, Vec<String>>>;

    type Error = Error;

    fn decode_message(message: Message<Self::Body>) -> Result<Self, Self::Error> {
        match message.body {
            Some(body) => Ok(Self { body }),
            None => Ok(Self {
                body: OrderedMap::with_capacity(0),
            }),
        }
    }
}
