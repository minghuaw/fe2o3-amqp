//! Extension trait for AMQP messages to provide management specific functionality

use fe2o3_amqp_types::{
    messaging::{Message, MessageId},
    primitives::SimpleValue,
};

use crate::{constants, error::InvalidType, status::StatusCode};

/// Extension trait for AMQP messages to provide management specific functionality
pub trait AmqpMessageManagementExt {
    /// Get the status code from the message
    fn status_code(&self) -> Option<Result<StatusCode, InvalidType>>;

    /// Remove the status code from the message
    fn remove_status_code(&mut self) -> Option<Result<StatusCode, SimpleValue>>;

    /// Get the correlation id from the message
    fn correlation_id(&self) -> Option<&MessageId>;

    /// Remove the correlation id from the message
    fn remove_correlation_id(&mut self) -> Option<MessageId>;

    /// Get the status description from the message
    fn status_description(&self) -> Option<Result<&str, InvalidType>>;

    /// Remove the status description from the message
    fn remove_status_description(&mut self) -> Option<Result<String, SimpleValue>>;
}

impl<T> AmqpMessageManagementExt for Message<T> {
    fn status_code(&self) -> Option<Result<StatusCode, InvalidType>> {
        self.application_properties
            .as_ref()
            .and_then(|ap| {
                ap.get(constants::kebab_case::STATUS_CODE)
                    .or_else(|| ap.get(constants::lower_camel_case::STATUS_CODE))
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
                ap.swap_remove(constants::kebab_case::STATUS_CODE)
                    .or_else(|| ap.swap_remove(constants::lower_camel_case::STATUS_CODE))
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
                .or_else(|| ap.get(constants::lower_camel_case::STATUS_DESCRIPTION))
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
            ap.swap_remove(constants::kebab_case::STATUS_DESCRIPTION)
                .or_else(|| ap.swap_remove(constants::lower_camel_case::STATUS_DESCRIPTION))
                .map(|value| match value {
                    SimpleValue::String(s) => Ok(s),
                    _ => Err(value),
                })
        })
    }
}
