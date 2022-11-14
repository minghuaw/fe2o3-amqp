use std::borrow::Cow;

use fe2o3_amqp_types::{messaging::Message, primitives::OrderedMap};

use crate::{
    constants::{GET_ANNOTATIONS, OPERATION},
    error::{Error, Result},
    request::Request,
    response::Response,
};

use super::get::GetRequest;

pub trait GetAnnotations {
    fn get_annotations(&self, req: GetAnnotationsRequest) -> Result<GetAnnotationsResponse>;
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
    type Response = GetAnnotationsResponse;
    type Body = ();

    fn into_message(self) -> Message<Self::Body> {
        let mut application_properties = self.inner.into_application_properties();
        application_properties.insert(OPERATION.into(), GET_ANNOTATIONS.into());

        Message::builder()
            .application_properties(application_properties)
            .body(())
            .build()
    }
}

pub struct GetAnnotationsResponse {
    pub annotations: OrderedMap<String, Vec<String>>,
}

impl Response for GetAnnotationsResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Option<OrderedMap<String, Vec<String>>>;

    type Error = Error;
    type StatusError = Error;

    fn from_message(mut message: Message<Option<OrderedMap<String, Vec<String>>>>) -> Result<Self> {
        let _status_code = Self::check_status_code(&mut message)?;

        match message.body {
            Some(annotations) => Ok(Self { annotations }),
            None => Ok(Self {
                annotations: OrderedMap::with_capacity(0),
            }),
        }
    }
}
