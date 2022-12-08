//! Defines the Response trait for AMQP 1.0 management responses.

use fe2o3_amqp_types::messaging::{FromBody, Message};

use crate::{
    error::{InvalidType, StatusCodeNotFound, StatusError},
    mgmt_ext::AmqpMessageManagementExt,
    status::StatusCode,
};

/// A trait for AMQP 1.0 management response.
pub trait Response: Sized {
    /// The status code of the response.
    const STATUS_CODE: u16;

    /// The body type of the response.
    type Body: for<'de> FromBody<'de>;

    /// The error type of the response.
    type Error: From<StatusError> + From<InvalidType> + From<StatusCodeNotFound>;

    /// Decodes the response from the message.
    /// 
    /// This is the only function that the user needs to implement in most cases. The full 
    /// response decoding takes place in the [`from_message`] function, please see [`from_message`]
    /// for more details.
    fn decode_message(message: Message<Self::Body>) -> Result<Self, Self::Error>;

    /// Checks the status code and description of the response and returns `None` if status code is
    /// not found or returns `Some(Err(error))` if the status code is not the expected one.
    ///
    /// Please note that the blanket implementation will remove the status code and description.
    /// The user should override the blanket implementation if they want to keep the status code
    /// and description.
    fn verify_status_code(message: &mut Message<Self::Body>) -> Result<StatusCode, Self::Error> {
        let status_code = match message.remove_status_code().ok_or(StatusCodeNotFound {})? {
            Ok(status_code) => status_code,
            Err(err) => {
                return Err(InvalidType {
                    expected: "u16".to_string(),
                    actual: format!("{:?}", err),
                }
                .into())
            }
        };
        if status_code.0.get() != Self::STATUS_CODE {
            let status_description = match message.remove_status_description() {
                Some(Ok(status_description)) => Some(status_description),
                Some(Err(err)) => {
                    return Err(InvalidType {
                        expected: "String".to_string(),
                        actual: format!("{:?}", err),
                    }
                    .into())
                }
                None => None,
            };

            return Err(StatusError {
                code: status_code,
                description: status_description.map(Into::into),
            }
            .into());
        }

        Ok(status_code)
    }

    /// Decodes the response from the message.
    /// 
    /// The blanket implementation simply calls [`verify_status_code`] and [`decode_message`],
    /// which should work for most cases. The user should override this function if more than one
    /// successful status code is expected.
    fn from_message(mut message: Message<Self::Body>) -> Result<Self, Self::Error> {
        Self::verify_status_code(&mut message)?;
        Self::decode_message(message)
    }
}
