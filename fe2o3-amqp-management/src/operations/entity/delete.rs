use std::collections::BTreeMap;

use fe2o3_amqp_types::primitives::Value;

use crate::{error::Result};

pub trait Delete {
    fn delete(&mut self, arg: DeleteRequest) -> Result<DeleteResponse>;
}

pub struct EmptyBTreeMap(BTreeMap<Value, Value>);

impl EmptyBTreeMap {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
}

/// Delete a Manageable Entity.
///
/// # Body:
/// 
/// No information is carried in the message body therefore any message body is valid and MUST be
/// ignored.
pub struct DeleteRequest {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    pub name: String,

    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    pub identity: String,
}

/// The body of the message MUST consist of an amqp-value section containing a map with zero
/// entries. If the request was successful then the statusCode MUST be 204 (No Content).
pub struct DeleteResponse {
    empty_map: EmptyBTreeMap,
}

impl DeleteResponse {
    const STATUS_CODE: u16 = 204;
}