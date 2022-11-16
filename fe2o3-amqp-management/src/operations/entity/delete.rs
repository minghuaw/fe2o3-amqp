use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::{OrderedMap, SimpleValue, Value},
};

use crate::{
    constants::{DELETE, IDENTITY, NAME},
    error::{Error, InvalidType},
    request::Request,
    response::Response,
};

pub trait Delete {
    fn delete(&mut self, arg: DeleteRequest) -> Result<DeleteResponse, Error>;
}

pub struct EmptyMap(OrderedMap<String, Value>);

impl EmptyMap {
    pub fn new() -> Self {
        Self(OrderedMap::with_capacity(0))
    }
}

impl Default for EmptyMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Delete a Manageable Entity.
///
/// # Body:
///
/// No information is carried in the message body therefore any message body is valid and MUST be
/// ignored.
pub enum DeleteRequest<'a> {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    Name {
        value: Cow<'a, str>,
        r#type: Cow<'a, str>,
        locales: Option<Cow<'a, str>>,
    },

    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    Identity {
        value: Cow<'a, str>,
        r#type: Cow<'a, str>,
        locales: Option<Cow<'a, str>>,
    },
}

impl<'a> DeleteRequest<'a> {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    pub fn name(
        value: impl Into<Cow<'a, str>>,
        r#type: impl Into<Cow<'a, str>>,
        locales: impl Into<Option<Cow<'a, str>>>,
    ) -> Self {
        Self::Name {
            value: value.into(),
            r#type: r#type.into(),
            locales: locales.into(),
        }
    }

    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    pub fn identity(
        value: impl Into<Cow<'a, str>>,
        r#type: impl Into<Cow<'a, str>>,
        locales: impl Into<Option<Cow<'a, str>>>,
    ) -> Self {
        Self::Identity {
            value: value.into(),
            r#type: r#type.into(),
            locales: locales.into(),
        }
    }
}

impl<'a> Request for DeleteRequest<'a> {
    const OPERATION: &'static str = DELETE;

    type Response = DeleteResponse;

    type Body = ();

    fn manageable_entity_type(&mut self) -> Option<String> {
        match self {
            Self::Name { r#type, .. } => Some(r#type.to_string()),
            Self::Identity { r#type, .. } => Some(r#type.to_string()),
        }
    }

    fn locales(&mut self) -> Option<String> {
        match self {
            Self::Name { locales, .. } => locales.as_ref().map(|s| s.to_string()),
            Self::Identity { locales, .. } => locales.as_ref().map(|s| s.to_string()),
        }
    }

    fn encode_application_properties(&mut self) -> Option<ApplicationProperties> {
        let (key, value) = match self {
            Self::Name { value, .. } => (NAME, value),
            Self::Identity { value, .. } => (IDENTITY, value),
        };

        let app_props = ApplicationProperties::builder()
            .insert(key, SimpleValue::String(value.to_string()))
            .build();
        Some(app_props)
    }

    fn encode_body(self) -> Self::Body {}
}

/// The body of the message MUST consist of an amqp-value section containing a map with zero
/// entries. If the request was successful then the statusCode MUST be 204 (No Content).
pub struct DeleteResponse {
    pub empty_map: EmptyMap,
}

impl DeleteResponse {}

impl Response for DeleteResponse {
    const STATUS_CODE: u16 = 204;

    type Body = Option<OrderedMap<String, Value>>;

    type Error = Error;

    fn decode_message(message: Message<Self::Body>) -> Result<Self, Self::Error> {
        match message.body.map(|m| m.len()) {
            None | Some(0) => Ok(Self {
                empty_map: EmptyMap::new(),
            }),
            _ => Err(Error::DecodeError(
                InvalidType {
                    expected: "empty map".to_string(),
                    actual: "non-empty map".to_string(),
                }
                .into(),
            )),
        }
    }
}
