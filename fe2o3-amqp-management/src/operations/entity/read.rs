use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::{OrderedMap, Value},
};

use crate::{
    constants::{IDENTITY, NAME, READ},
    error::Error,
    request::Request,
    response::Response,
};

/// A trait for handling Read request on a Manageable Entity.
pub trait Read {
    /// Handles a Read request.
    fn read(&mut self, arg: ReadRequest) -> Result<ReadResponse, Error>;
}

/// Retrieve the attributes of a Manageable Entity.
///
/// Exactly one of name or identity MUST be provided
///
/// Body: No information is carried in the message body therefore any message body is valid and MUST
/// be ignored
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadRequest<'a> {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    Name {
        /// The name of the Manageable Entity to be managed. This is case-sensitive.
        value: Cow<'a, str>,
        /// The type of the Manageable Entity to be managed. This is case-sensitive.
        r#type: Cow<'a, str>,
        /// The locales to be used for any error messages. This is case-sensitive.
        locales: Option<Cow<'a, str>>,
    },

    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    Identity {
        /// The identity of the Manageable Entity to be managed. This is case-sensitive.
        value: Cow<'a, str>,
        /// The type of the Manageable Entity to be managed. This is case-sensitive.
        r#type: Cow<'a, str>,
        /// The locales to be used for any error messages. This is case-sensitive.
        locales: Option<Cow<'a, str>>,
    },
}

impl<'a> ReadRequest<'a> {
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

impl<'a> Request for ReadRequest<'a> {
    const OPERATION: &'static str = READ;

    type Response = ReadResponse;

    type Body = ();

    fn manageable_entity_type(&mut self) -> Option<String> {
        match self {
            ReadRequest::Name { r#type, .. } => Some(r#type.to_string()),
            ReadRequest::Identity { r#type, .. } => Some(r#type.to_string()),
        }
    }

    fn locales(&mut self) -> Option<String> {
        match self {
            ReadRequest::Name { locales, .. } => locales.as_ref().map(|s| s.to_string()),
            ReadRequest::Identity { locales, .. } => locales.as_ref().map(|s| s.to_string()),
        }
    }

    fn encode_application_properties(&mut self) -> Option<ApplicationProperties> {
        let (key, value) = match self {
            ReadRequest::Name { value, .. } => (NAME, value),
            ReadRequest::Identity { value, .. } => (IDENTITY, value),
        };

        Some(
            ApplicationProperties::builder()
                .insert(key, &value[..])
                .build(),
        )
    }

    fn encode_body(self) -> Self::Body {}
}

/// The response to a Read request.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReadResponse {
    /// The attributes of the Manageable Entity.
    pub entity_attributes: OrderedMap<String, Value>,
}

impl ReadResponse {}

impl Response for ReadResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Option<OrderedMap<String, Value>>;
    type Error = Error;

    fn decode_message(message: Message<Self::Body>) -> Result<Self, Self::Error> {
        match message.body {
            Some(map) => Ok(Self {
                entity_attributes: map,
            }),
            None => Ok(Self {
                entity_attributes: OrderedMap::with_capacity(0),
            }),
        }
    }
}
