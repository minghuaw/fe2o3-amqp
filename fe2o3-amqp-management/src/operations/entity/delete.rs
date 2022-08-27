use crate::{Extractor, IntoResponse, error::Result};

pub trait Delete {
    type Req: Extractor;
    type Res: IntoResponse;

    fn delete(&mut self, arg: Self::Req) -> Result<Self::Res>;
}

pub struct DeleteRequestProperties {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    pub name: String,

    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    pub identity: String,
}

/// The body of the message MUST consist of an amqp-value section containing a map with zero
/// entries. If the request was successful then the statusCode MUST be 204 (No Content).
pub struct DeleteResponse {

}

impl DeleteResponse {
    const STATUS_CODE: u16 = 204;
}