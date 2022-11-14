use fe2o3_amqp_types::messaging::{FromBody, Message};

use crate::{
    error::{InvalidType, StatusCodeNotFound, StatusError},
    mgmt_ext::AmqpMessageManagementExt,
    status::StatusCode,
};

/// Leaving checking particular field of response entirely to the user. A blanker implementation of
/// checking the status code is provided in the `check_status_code` method; however, the user will 
/// need to decide whether to use it or not.
/// 
/// This will likely see some significant changes in the future.
///
/// TODO: only allowing one type of body for now as it seems like all responses should know the
/// exact type of body they are expecting
pub trait Response: Sized {
    const STATUS_CODE: u16;

    type Body: for<'de> FromBody<'de>;

    type Error;
    type StatusError: From<StatusError> + From<InvalidType> + From<StatusCodeNotFound>;

    fn from_message(message: Message<Self::Body>) -> Result<Self, Self::Error>;

    /// Checks the status code and description of the response and returns `None` if status code is
    /// not found or returns `Some(Err(error))` if the status code is not the expected one.
    fn check_status_code(
        message: &mut Message<Self::Body>,
    ) -> Result<StatusCode, Self::StatusError> {
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
}
