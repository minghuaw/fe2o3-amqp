use std::collections::BTreeMap;

use crate::{Extractor, IntoResponse, error::Result, status::StatusCode};

pub trait Create {
    type Req: Extractor;
    type Res: IntoResponse;

    fn create(&mut self, req: Self::Req) -> Result<Self::Res>;
}

pub struct CreateRequestProperties {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    pub name: String,
}

/// If the request was successful then the statusCode MUST be 201 (Created) and the body of the
/// message MUST consist of an amqp-value section that contains a map containing the actual
/// attributes of the entity created. These MAY differ from those requested in two ways:
/// 
///     • Default values may be returned for values not specified
///     • Specific/concrete values may be returned for generic/base values specified
///     • The value associated with an attribute may have been converted into the correct amqp type
/// 
/// (e.g. the string “2” into the integer value 2) A map containing attributes that are not
/// applicable for the entity being created, or invalid values for a given attribute, MUST result in
/// a failure response with a statusCode of 400 (Bad Request).
pub struct CreateResponse {
    entity_attributes: BTreeMap<String, String>,
}

impl CreateResponse {
    const STATUS_CODE: u16 = 201;
}