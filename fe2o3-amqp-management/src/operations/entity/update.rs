use std::collections::BTreeMap;

use crate::{Extractor, IntoResponse, error::Result};

pub trait Update {
    type Req: Extractor;
    type Res: IntoResponse;

    fn update(&mut self, arg: Self::Req) -> Result<Self::Res>;
}

pub struct UpdateRequestProperties {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    pub name: String,

    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    pub identity: String,
}

/// If the request was successful then the statusCode MUST contain 200 (OK) and the body of the
/// message MUST consists of an amqp-value section containing a map of the actual attributes of the
/// entity updated. These MAY differ from those requested. 
/// 
/// A map containing attributes that are not
/// applicable for the entity being created, or an invalid value for a given attribute (excepting
/// type conversion as above), MUST result in a failure response with a statusCode of 400 (Bad
/// Request).
pub struct UpdateResponse {
    entity_attributes: BTreeMap<String, String>,
}

impl UpdateResponse {
    const STATUS_CODE: u16 = 200;
}