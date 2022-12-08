use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::{OrderedMap, Value},
};

use crate::{
    constants::{IDENTITY, NAME, UPDATE},
    error::Error,
    request::Request,
    response::Response,
};

/// A trait for handling Update request on a Manageable Entity.
pub trait Update {
    /// Handles a Update request.
    fn update(&mut self, arg: UpdateRequest) -> Result<UpdateResponse, Error>;
}

/// Update a Manageable Entity.
///
/// # Body:
///
/// The body MUST consist of an amqp-value section containing a map. The map consists of key-value
/// pairs where the key represents the name of an attribute of the entity and the value represents
/// the initial value it SHOULD take. The absence of an attribute name implies that the entity
/// should retain its existing value.
///
/// If the map contains a key-value pair where the value is null then the updated entity should have
/// no value for that attribute, removing any previous value.
///
/// In the case where the supplied map contains multiple attributes, then the update MUST be treated
/// as a single, atomic operation so if any of the changes cannot be applied, none of the attributes
/// in the map should be updated and this MUST result in a failure response.
///
/// Where the type of the attribute value provided is not as required, type conversion as per the
/// rules in 3.3.1.1 MUST be provided.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UpdateRequest<'a> {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    Name {
        /// The name of the Manageable Entity to be managed. This is case-sensitive.
        value: Cow<'a, str>,
        /// Entity type
        r#type: Cow<'a, str>,
        /// The locales to be used for any error messages. This is case-sensitive.
        locales: Option<Cow<'a, str>>,
        /// The body MUST consist of an amqp-value section containing a map.
        body: OrderedMap<String, Value>,
    },
    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    Identity {
        /// The identity of the Manageable Entity to be managed. This is case-sensitive.
        value: Cow<'a, str>,
        /// Entity type
        r#type: Cow<'a, str>,
        /// The locales to be used for any error messages. This is case-sensitive.
        locales: Option<Cow<'a, str>>,
        /// The body MUST consist of an amqp-value section containing a map.
        body: OrderedMap<String, Value>,
    },
}

impl<'a> UpdateRequest<'a> {
    /// Creates a new UpdateRequest with the entity name.
    pub fn name(
        name: impl Into<Cow<'a, str>>,
        r#type: impl Into<Cow<'a, str>>,
        locales: impl Into<Option<Cow<'a, str>>>,
        body: impl Into<OrderedMap<String, Value>>,
    ) -> Self {
        Self::Name {
            value: name.into(),
            r#type: r#type.into(),
            locales: locales.into(),
            body: body.into(),
        }
    }

    /// Creates a new UpdateRequest with the entity identity.
    pub fn identity(
        identity: impl Into<Cow<'a, str>>,
        r#type: impl Into<Cow<'a, str>>,
        locales: impl Into<Option<Cow<'a, str>>>,
        body: impl Into<OrderedMap<String, Value>>,
    ) -> Self {
        Self::Identity {
            value: identity.into(),
            r#type: r#type.into(),
            locales: locales.into(),
            body: body.into(),
        }
    }
}

impl<'a> Request for UpdateRequest<'a> {
    const OPERATION: &'static str = UPDATE;

    type Response = UpdateResponse;

    type Body = OrderedMap<String, Value>;

    fn manageable_entity_type(&mut self) -> Option<String> {
        match self {
            UpdateRequest::Name { r#type, .. } => Some(r#type.to_string()),
            UpdateRequest::Identity { r#type, .. } => Some(r#type.to_string()),
        }
    }

    fn locales(&mut self) -> Option<String> {
        match self {
            UpdateRequest::Name { locales, .. } => locales.as_ref().map(|s| s.to_string()),
            UpdateRequest::Identity { locales, .. } => locales.as_ref().map(|s| s.to_string()),
        }
    }

    fn encode_application_properties(&mut self) -> Option<ApplicationProperties> {
        let (key, value) = match self {
            UpdateRequest::Name { value, .. } => (NAME, value),
            UpdateRequest::Identity { value, .. } => (IDENTITY, value),
        };

        Some(
            ApplicationProperties::builder()
                .insert(key, &value[..])
                .build(),
        )
    }

    fn encode_body(self) -> Self::Body {
        match self {
            UpdateRequest::Name { body, .. } => body,
            UpdateRequest::Identity { body, .. } => body,
        }
    }
}

/// If the request was successful then the statusCode MUST contain 200 (OK) and the body of the
/// message MUST consists of an amqp-value section containing a map of the actual attributes of the
/// entity updated. These MAY differ from those requested.
///
/// A map containing attributes that are not
/// applicable for the entity being created, or an invalid value for a given attribute (excepting
/// type conversion as above), MUST result in a failure response with a statusCode of 400 (Bad
/// Request).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UpdateResponse {
    /// The entity attributes.
    pub entity_attributes: OrderedMap<String, Value>,
}

impl Response for UpdateResponse {
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
