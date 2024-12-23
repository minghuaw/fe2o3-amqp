use std::borrow::Cow;

use fe2o3_amqp_types::{messaging::Message, primitives::OrderedMap};

use crate::{constants::GET_TYPES, error::Error, request::Request, response::Response};

use super::get::GetRequest;

/// A trait for handling GetTypes request on a Manageable Node.
pub trait GetTypes {
    /// Handles a GetTypes request.
    fn get_types(&self, req: GetTypesRequest) -> Result<GetTypesResponse, Error>;
}

/// GET-TYPES
///
/// Body:
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetTypesRequest<'a> {
    inner: GetRequest<'a>,
}

impl<'a> GetTypesRequest<'a> {
    /// Creates a new GetTypes request.
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

impl Request for GetTypesRequest<'_> {
    const OPERATION: &'static str = GET_TYPES;

    type Response = GetTypesResponse;

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

/// The response to a GetTypes request.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetTypesResponse {
    /// The types of the manageable entities.
    pub types: OrderedMap<String, Vec<String>>,
}

impl GetTypesResponse {}

impl Response for GetTypesResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Option<OrderedMap<String, Vec<String>>>;

    type Error = Error;

    fn decode_message(message: Message<Self::Body>) -> Result<Self, Self::Error> {
        match message.body {
            Some(types) => Ok(Self { types }),
            None => Ok(Self {
                types: OrderedMap::with_capacity(0),
            }),
        }
    }
}
