use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::{OrderedMap, SimpleValue, Value},
};

use crate::{
    constants::{IDENTITY, LOCALES, NAME, OPERATION, TYPE, UPDATE},
    error::{Error, Result},
    request::Request,
    response::Response,
};

pub trait Update {
    fn update(&mut self, arg: UpdateRequest) -> Result<UpdateResponse>;
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
pub enum UpdateRequest<'a> {
    Name {
        value: Cow<'a, str>,
        r#type: Cow<'a, str>,
        locales: Option<Cow<'a, str>>,
        body: OrderedMap<String, Value>,
    },
    Identity {
        value: Cow<'a, str>,
        r#type: Cow<'a, str>,
        locales: Option<Cow<'a, str>>,
        body: OrderedMap<String, Value>,
    },
}

impl<'a> UpdateRequest<'a> {
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
    type Response = UpdateResponse;
    type Body = OrderedMap<String, Value>;

    fn into_message(self) -> Message<Self::Body> {
        let (key, value, body, r#type, locales) = match self {
            UpdateRequest::Name {
                value,
                body,
                r#type,
                locales,
            } => (NAME, value, body, r#type, locales),
            UpdateRequest::Identity {
                value,
                body,
                r#type,
                locales,
            } => (IDENTITY, value, body, r#type, locales),
        };

        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, UPDATE)
                    .insert(TYPE, r#type.to_string())
                    .insert(
                        LOCALES,
                        locales
                            .map(|s| SimpleValue::from(s.to_string()))
                            .unwrap_or(SimpleValue::Null),
                    )
                    .insert(key, &value[..])
                    .build(),
            )
            .body(body)
            .build()
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
pub struct UpdateResponse {
    pub entity_attributes: OrderedMap<String, Value>,
}

impl Response for UpdateResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Option<OrderedMap<String, Value>>;

    type Error = Error;
    type StatusError = Error;

    fn from_message(mut message: Message<Option<OrderedMap<String, Value>>>) -> Result<Self> {
        let _status_code = Self::check_status_code(&mut message)?;
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
