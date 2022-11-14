use fe2o3_amqp_types::messaging::{FromBody, Message, MessageId};

use crate::{
    constants::{STATUS_CODE, STATUS_DESCRIPTION},
    error::{Error, StatusError, InvalidType, StatusCodeNotFound},
    status::StatusCode, mgmt_ext::AmqpMessageManagementExt,
};

// /// The correlation-id of the response message MUST be the correlation-id from the request message
// /// (if present), else the message-id from the request message. Response messages have the following
// /// application-properties:
// pub struct ResponseMessageProperties {
//     pub correlation_id: MessageId,
//     pub status_code: StatusCode,
//     pub status_description: Option<String>,
// }

// impl ResponseMessageProperties {
//     pub fn try_take_from_message<T>(message: &mut Message<T>) -> Result<Self, Error> {
//         let correlation_id = match message
//             .properties
//             .as_mut()
//             .and_then(|p| p.correlation_id.take())
//         {
//             Some(correlation_id) => correlation_id,
//             None => message
//                 .properties
//                 .as_mut()
//                 .and_then(|p| p.message_id.take())
//                 .ok_or(Error::CorrelationIdAndMessageIdAreNone)?,
//         };

//         let status_code = match message
//             .application_properties
//             .as_mut()
//             .and_then(|ap| ap.remove(STATUS_CODE))
//         {
//             Some(value) => StatusCode::try_from(value).map_err(|_| Error::DecodeError)?,
//             None => return Err(Error::StatusCodeNotFound),
//         };

//         let status_description: Option<String> = message
//             .application_properties
//             .as_mut()
//             .and_then(|ap| ap.remove(STATUS_DESCRIPTION))
//             .map(|value| String::try_from(value).map_err(|_| Error::DecodeError))
//             .transpose()?;

//         Ok(Self {
//             correlation_id,
//             status_code,
//             status_description,
//         })
//     }
// }

// #[derive(Debug)]
// pub struct Response<R> {
//     pub correlation_id: MessageId,
//     pub status_code: StatusCode,
//     pub status_description: Option<String>,
//     pub operation: R,
// }

// impl<R> Response<R> {
//     pub fn from_parts(properties: ResponseMessageProperties, operation: R) -> Self {
//         Self {
//             correlation_id: properties.correlation_id,
//             status_code: properties.status_code,
//             status_description: properties.status_description,
//             operation,
//         }
//     }
// }

/// Leaving checking particular field of response entirely to the user.
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
    fn check_status_code(message: &mut Message<Self::Body>) -> Result<StatusCode, Self::StatusError> {
        let status_code = match message.remove_status_code().ok_or(StatusCodeNotFound {})? {
            Ok(status_code) => status_code,
            Err(err) => return Err(InvalidType {
                expected: "u16".to_string(),
                actual: format!("{:?}", err),
            }.into()),
        };
        if status_code.0.get() != Self::STATUS_CODE {
            let status_description = match message
                .remove_status_description()
            {
                Some(Ok(status_description)) => Some(status_description),
                Some(Err(err)) => return Err(InvalidType {
                    expected: "String".to_string(),
                    actual: format!("{:?}", err),
                }.into()),
                None => None,
            };

            return Err(StatusError {
                code: status_code,
                description: status_description.map(Into::into),
            }.into());
        }

        Ok(status_code)
    }
}
