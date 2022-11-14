use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::OrderedMap,
};

use crate::{
    constants::{GET_OPERATIONS, OPERATION},
    error::{Error, Result}, request::Request, response::Response,
};

use super::get::GetRequest;

pub trait GetOperations {
    fn get_operations(&self, req: GetOperationsRequest) -> Result<GetOperationsResponse>;
}

/// GET-OPERATIONS
///
/// Retrieve the list of Management Operations (and the arguments they take) which can be performed
/// via this Management Node.
///
/// Body
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct GetOperationsRequest<'a> {
    inner: GetRequest<'a>,
}

impl<'a> GetOperationsRequest<'a> {
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
    type Response = GetOperationsResponse;
    type Body = ();

    fn into_message(self) -> Message<Self::Body> {
        let mut application_properties = self.inner.into_application_properties();
        application_properties.insert(OPERATION.into(), GET_OPERATIONS.into());

        Message::builder()
            .application_properties(application_properties)
            .body(())
            .build()
    }
}

type Operations = OrderedMap<String, OrderedMap<String, Vec<String>>>;

/// If the request was successful then the statusCode MUST be 200 (OK) and the body of the message
/// MUST consist of an amqp-value section containing a map. The keys in the map MUST be the set of
/// Manageable Entity Types for which the list of Management Operations is being provided. For any
/// given key, the value MUST itself be a map, where each key is the string name of a Management
/// Operation that can be performed against this Manageable Entity Type via this Management Node,
/// and the value for a given key is a list of strings giving the names of the arguments (passed via
/// the application- properties of a request message) which the operation defines. For any given
/// Manageable Entity Type, the set of operations returned MUST include every operation supported by
/// Manageable Entity Types that it extends, either directly or indirectly.
pub struct GetOperationsResponse {
    pub operations: Operations,
}

impl GetOperationsResponse {
}

impl Response for GetOperationsResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Option<Operations>;

    type Error = Error;
    type StatusError = Error;

    fn from_message(
        mut message: Message<Option<Operations>>,
    ) -> Result<Self> {
        let _status_code = Self::check_status_code(&mut message)?;

        match message.body {
            Some(operations) => Ok(Self { operations }),
            None => Ok(Self {
                operations: Operations::with_capacity(0),
            }),
        }
    }
}
