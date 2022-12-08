use std::borrow::Cow;

use fe2o3_amqp_types::{messaging::Message, primitives::OrderedMap};

use crate::{constants::GET_OPERATIONS, error::Error, request::Request, response::Response};

use super::get::GetRequest;

/// A trait for handling GetOperations request on a Manageable Node.
pub trait GetOperations {
    /// Handles a GetOperations request.
    fn get_operations(&self, req: GetOperationsRequest) -> Result<GetOperationsResponse, Error>;
}

/// GET-OPERATIONS
///
/// Retrieve the list of Management Operations (and the arguments they take) which can be performed
/// via this Management Node.
///
/// Body
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetOperationsRequest<'a> {
    inner: GetRequest<'a>,
}

impl<'a> GetOperationsRequest<'a> {
    /// Creates a new GetOperations request.
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

impl<'a> Request for GetOperationsRequest<'a> {
    const OPERATION: &'static str = GET_OPERATIONS;

    type Response = GetOperationsResponse;

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

type GetOperationsResponseBody = OrderedMap<String, OrderedMap<String, Vec<String>>>;

/// If the request was successful then the statusCode MUST be 200 (OK) and the body of the message
/// MUST consist of an amqp-value section containing a map. The keys in the map MUST be the set of
/// Manageable Entity Types for which the list of Management Operations is being provided. For any
/// given key, the value MUST itself be a map, where each key is the string name of a Management
/// Operation that can be performed against this Manageable Entity Type via this Management Node,
/// and the value for a given key is a list of strings giving the names of the arguments (passed via
/// the application- properties of a request message) which the operation defines. For any given
/// Manageable Entity Type, the set of operations returned MUST include every operation supported by
/// Manageable Entity Types that it extends, either directly or indirectly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetOperationsResponse {
    /// The response body.
    pub body: GetOperationsResponseBody,
}

impl GetOperationsResponse {}

impl Response for GetOperationsResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Option<GetOperationsResponseBody>;

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
