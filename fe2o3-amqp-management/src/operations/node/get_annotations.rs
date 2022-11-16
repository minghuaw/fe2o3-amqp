use std::borrow::Cow;

use fe2o3_amqp_types::{messaging::Message, primitives::OrderedMap};

use crate::{constants::GET_ANNOTATIONS, error::Error, request::Request, response::Response};

use super::get::GetRequest;

pub trait GetAnnotations {
    fn get_annotations(&self, req: GetAnnotationsRequest) -> Result<GetAnnotationsResponse, Error>;
}

/// GET-ANNOTATIONS
///
/// Body:
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct GetAnnotationsRequest<'a> {
    inner: GetRequest<'a>,
}

impl<'a> GetAnnotationsRequest<'a> {
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

impl<'a> Request for GetAnnotationsRequest<'a> {
    const OPERATION: &'static str = GET_ANNOTATIONS;

    type Response = GetAnnotationsResponse;

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

pub struct GetAnnotationsResponse {
    pub annotations: OrderedMap<String, Vec<String>>,
}

impl Response for GetAnnotationsResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Option<OrderedMap<String, Vec<String>>>;

    type Error = Error;

    fn decode_message(message: Message<Self::Body>) -> Result<Self, Self::Error> {
        match message.body {
            Some(annotations) => Ok(Self { annotations }),
            None => Ok(Self {
                annotations: OrderedMap::with_capacity(0),
            }),
        }
    }
}
