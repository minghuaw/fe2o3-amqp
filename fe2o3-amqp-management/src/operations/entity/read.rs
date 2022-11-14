use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message, MessageId},
    primitives::{OrderedMap, SimpleValue, Value},
};

use crate::{
    constants::{IDENTITY, LOCALES, NAME, OPERATION, READ, TYPE},
    error::{Error, Result},
    request::Request,
    response::Response, mgmt_ext::AmqpMessageManagementExt,
};

pub trait Read {
    fn read(&mut self, arg: ReadRequest) -> Result<ReadResponse>;
}

/// Retrieve the attributes of a Manageable Entity.
///
/// Exactly one of name or identity MUST be provided
///
/// Body: No information is carried in the message body therefore any message body is valid and MUST
/// be ignored
#[derive(Debug)]
pub enum ReadRequest<'a> {
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
    type Response = ReadResponse;
    type Body = ();

    fn into_message(self) -> Message<Self::Body> {
        let (key, value, r#type, locales) = match self {
            ReadRequest::Name {
                value,
                r#type,
                locales,
            } => (NAME, value, r#type, locales),
            ReadRequest::Identity {
                value,
                r#type,
                locales,
            } => (IDENTITY, value, r#type, locales),
        };

        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, READ)
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

#[derive(Debug)]
pub struct ReadResponse {
    pub entity_attributes: OrderedMap<String, Value>,
}

impl ReadResponse {
}

impl Response for ReadResponse {
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
