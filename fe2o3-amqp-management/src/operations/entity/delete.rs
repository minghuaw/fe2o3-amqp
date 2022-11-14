use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message, MessageId},
    primitives::{OrderedMap, Value, SimpleValue},
};

use crate::{
    constants::{DELETE, IDENTITY, NAME, OPERATION, TYPE, LOCALES},
    error::{Error, Result, InvalidType}, request::Request, response::Response, mgmt_ext::AmqpMessageManagementExt,
};

pub trait Delete {
    fn delete(&mut self, arg: DeleteRequest) -> Result<DeleteResponse>;
}

pub struct EmptyMap(OrderedMap<String, Value>);

impl EmptyMap {
    pub fn new() -> Self {
        Self(OrderedMap::with_capacity(0))
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
    type Response = DeleteResponse;
    type Body = ();

    fn into_message(self) -> fe2o3_amqp_types::messaging::Message<Self::Body> {
        let (key, value, r#type, locales) = match self {
            Self::Name {
                value,
                r#type,
                locales,
            } => (NAME, value, r#type, locales),
            Self::Identity {
                value,
                r#type,
                locales,
            } => (IDENTITY, value, r#type, locales),
        };

        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, DELETE)
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
            .body(())
            .build()
    }
}

/// The body of the message MUST consist of an amqp-value section containing a map with zero
/// entries. If the request was successful then the statusCode MUST be 204 (No Content).
pub struct DeleteResponse {
    pub empty_map: EmptyMap,
    pub correlation_id: Option<MessageId>,
}

impl DeleteResponse {
}

impl Response for DeleteResponse {
    const STATUS_CODE: u16 = 204;

    type Body = Option<OrderedMap<String, Value>>;

    type Error = Error;
    type StatusError = Error;

    fn from_message(mut message: Message<Option<OrderedMap<String, Value>>>) -> Result<Self> {
        let _status_code = Self::check_status_code(&mut message)?;
        let correlation_id = message.remove_correlation_id();

        match message.body.map(|m| m.len()) {
            None | Some(0) => Ok(Self {
                empty_map: EmptyMap::new(),
                correlation_id,
            }),
            _ => Err(Error::DecodeError(InvalidType {
                expected: "empty map".to_string(),
                actual: "non-empty map".to_string(),
            }.into())),
        }
    }
}
