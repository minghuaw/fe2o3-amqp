use fe2o3_amqp_types::messaging::{IntoBody, Message};

use crate::response::Response;

/// This leaves the responsibility of transforming a request to a Message entirely to the user. The
/// only field that will be set automatically is the message-id (if not set) which is a source for
/// the correlation-id of the response.
/// 
/// This will likely see some significant changes in the future.
/// 
/// TODO: separating setting entity type, operation, and locales from the request body?
pub trait Request {
    type Response: Response;
    type Body: IntoBody;

    fn into_message(self) -> Message<Self::Body>;
}
