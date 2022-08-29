use std::collections::BTreeMap;

use fe2o3_amqp_types::{primitives::Value, messaging::{ApplicationProperties, Message, AmqpValue}};

use crate::{error::{Result, Error}, request::MessageSerializer, operations::{OPERATION, DELETE}, response::MessageDeserializer};

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

impl MessageSerializer for DeleteRequest {
    type Body = ();

    fn into_message(self) -> fe2o3_amqp_types::messaging::Message<Self::Body> {
        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, DELETE)
                    .insert("name", self.name)
                    .insert("identity", self.identity)
                    .build()
            )
            .value(())
            .build()
    }
}

/// The body of the message MUST consist of an amqp-value section containing a map with zero
/// entries. If the request was successful then the statusCode MUST be 204 (No Content).
pub struct DeleteResponse {
    empty_map: EmptyBTreeMap,
}

impl DeleteResponse {
    const STATUS_CODE: u16 = 204;
}

impl MessageDeserializer<BTreeMap<String, Value>> for DeleteResponse {
    type Error = Error;

    fn from_message(message: Message<BTreeMap<String, Value>>) -> Result<Self> {
        match message.body {
            fe2o3_amqp_types::messaging::Body::Value(AmqpValue(map)) => {
                if map.len() > 0 {
                    Err(Error::DecodeError)
                } else {
                    Ok(Self { empty_map: EmptyBTreeMap::new() })
                }
            },
            _ => Err(Error::DecodeError)
        }
    }
}