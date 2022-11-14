use fe2o3_amqp_types::{
    messaging::{Message, MessageId},
    primitives::SimpleValue,
};

use crate::{
    constants::{STATUS_CODE, STATUS_DESCRIPTION},
    error::InvalidType,
    status::StatusCode,
};

pub trait AmqpMessageManagementExt {
    fn status_code(&self) -> Option<Result<StatusCode, InvalidType>>;

    fn remove_status_code(&mut self) -> Option<Result<StatusCode, SimpleValue>>;

    fn correlation_id(&self) -> Option<&MessageId>;

    fn remove_correlation_id(&mut self) -> Option<MessageId>;

    fn status_description(&self) -> Option<Result<&str, InvalidType>>;

    fn remove_status_description(&mut self) -> Option<Result<String, SimpleValue>>;
}

impl<T> AmqpMessageManagementExt for Message<T> {
    fn status_code(&self) -> Option<Result<StatusCode, InvalidType>> {
        self.application_properties
            .as_ref()
            .and_then(|ap| ap.get(STATUS_CODE))
            .map(|value| {
                StatusCode::try_from(value).map_err(|actual| InvalidType {
                    expected: "StatusCode".to_string(),
                    actual: format!("{:?}", actual),
                })
            })
    }

    fn remove_status_code(&mut self) -> Option<Result<StatusCode, SimpleValue>> {
        self.application_properties
            .as_mut()
            .and_then(|ap| ap.remove(STATUS_CODE))
            .map(|value| StatusCode::try_from(value).map_err(|actual| SimpleValue::from(actual)))
    }

    fn correlation_id(&self) -> Option<&MessageId> {
        self.properties
            .as_ref()
            .and_then(|p| p.correlation_id.as_ref())
    }

    fn remove_correlation_id(&mut self) -> Option<MessageId> {
        self.properties
            .as_mut()
            .and_then(|p| p.correlation_id.take())
    }

    fn status_description(&self) -> Option<Result<&str, InvalidType>> {
        self.application_properties.as_ref().and_then(|ap| {
            ap.get(STATUS_DESCRIPTION).and_then(|value| match value {
                SimpleValue::String(s) => Some(Ok(s.as_str())),
                _ => Some(Err(InvalidType {
                    expected: "String".to_string(),
                    actual: format!("{:?}", value),
                })),
            })
        })
    }

    fn remove_status_description(&mut self) -> Option<Result<String, SimpleValue>> {
        self.application_properties.as_mut().and_then(|ap| {
            ap.remove(STATUS_DESCRIPTION).and_then(|value| match value {
                SimpleValue::String(s) => Some(Ok(s)),
                _ => Some(Err(value)),
            })
        })
    }
}
