use fe2o3_amqp_types::messaging::{Message, MessageId};

use crate::{error::Error, status::StatusCode, constants::{STATUS_CODE, STATUS_DESCRIPTION}};

/// The correlation-id of the response message MUST be the correlation-id from the request message
/// (if present), else the message-id from the request message. Response messages have the following
/// application-properties:
pub struct ResponseMessageProperties {
    pub correlation_id: MessageId,
    pub status_code: StatusCode,
    pub status_description: Option<String>,
}

impl ResponseMessageProperties {
    pub fn try_take_from_message<T>(message: &mut Message<T>) -> Result<Self, Error> {
        let correlation_id = match message
            .properties
            .as_mut()
            .and_then(|p| p.correlation_id.take())
        {
            Some(correlation_id) => correlation_id,
            None => message
                .properties
                .as_mut()
                .and_then(|p| p.message_id.take())
                .ok_or(Error::CorrelationIdAndMessageIdAreNone)?,
        };

        let status_code = match message
            .application_properties
            .as_mut()
            .and_then(|ap| ap.remove(STATUS_CODE))
        {
            Some(value) => StatusCode::try_from(value).map_err(|_| Error::DecodeError)?,
            None => return Err(Error::StatusCodeNotFound),
        };

        let status_description: Option<String> = message
            .application_properties
            .as_mut()
            .and_then(|ap| ap.remove(STATUS_DESCRIPTION))
            .map(|value| String::try_from(value).map_err(|_| Error::DecodeError))
            .transpose()?;

        Ok(Self {
            correlation_id,
            status_code,
            status_description,
        })
    }
}

#[derive(Debug)]
pub struct Response<R> {
    pub correlation_id: MessageId,
    pub status_code: StatusCode,
    pub status_description: Option<String>,
    pub operation: R,
}

impl<R> Response<R> {
    pub fn from_parts(properties: ResponseMessageProperties, operation: R) -> Self {
        Self {
            correlation_id: properties.correlation_id,
            status_code: properties.status_code,
            status_description: properties.status_description,
            operation,
        }
    }
}

pub trait MessageDeserializer<T>: Sized
where
    for<'de> T: serde::de::Deserialize<'de>,
{
    type Error;

    fn from_message(message: Message<T>) -> Result<Self, Self::Error>;
}
