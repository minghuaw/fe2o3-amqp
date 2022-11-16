use fe2o3_amqp_types::{
    messaging::{Message, MessageId},
    primitives::SimpleValue,
};

use crate::{constants, error::InvalidType, status::StatusCode};

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
            .and_then(|ap| {
                ap.get(constants::kebab_case::STATUS_CODE)
                    .or(ap.get(constants::lower_camel_case::STATUS_CODE))
            })
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
            .and_then(|ap| {
                ap.remove(constants::kebab_case::STATUS_CODE)
                    .or(ap.remove(constants::lower_camel_case::STATUS_CODE))
            })
            .map(StatusCode::try_from)
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
            ap.get(constants::kebab_case::STATUS_DESCRIPTION)
                .or(ap.get(constants::lower_camel_case::STATUS_DESCRIPTION))
                .map(|value| match value {
                    SimpleValue::String(s) => Ok(s.as_str()),
                    _ => Err(InvalidType {
                        expected: "String".to_string(),
                        actual: format!("{:?}", value),
                    }),
                })
        })
    }

    fn remove_status_description(&mut self) -> Option<Result<String, SimpleValue>> {
        self.application_properties.as_mut().and_then(|ap| {
            ap.remove(constants::kebab_case::STATUS_DESCRIPTION)
                .or(ap.remove(constants::lower_camel_case::STATUS_DESCRIPTION))
                .map(|value| match value {
                    SimpleValue::String(s) => Ok(s),
                    _ => Err(value),
                })
        })
    }
}
