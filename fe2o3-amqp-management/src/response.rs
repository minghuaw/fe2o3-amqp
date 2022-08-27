use fe2o3_amqp_types::messaging::MessageId;

use crate::status::StatusCode;


/// The correlation-id of the response message MUST be the correlation-id from the request message
/// (if present), else the message-id from the request message. Response messages have the following
/// application-properties:
pub struct ResponseMessageProperties {
    pub correlation_id: MessageId,
    pub status_code: StatusCode,
    pub status_description: Option<String>
}