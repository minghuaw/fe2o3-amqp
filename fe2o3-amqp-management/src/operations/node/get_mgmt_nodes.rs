use fe2o3_amqp_types::messaging::{AmqpValue, ApplicationProperties, Body, Message};

use crate::{
    error::{Error, Result},
    constants::{GET_MGMT_NODES, OPERATION},
    request::MessageSerializer,
    response::MessageDeserializer,
};

pub trait GetMgmtNodes {
    fn get_mgmt_nodes(&self, req: GetMgmtNodesRequest) -> Result<GetMgmtNodesResponse>;
}

/// GET-MGMT-NODES
///
/// Retrieve the list of addresses of other Management Nodes which this Management Node is aware of.
///
/// Body:
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct GetMgmtNodesRequest {}

impl MessageSerializer for GetMgmtNodesRequest {
    type Body = ();

    fn into_message(self) -> fe2o3_amqp_types::messaging::Message<Self::Body> {
        Message::builder()
            .application_properties(
                ApplicationProperties::builder()
                    .insert(OPERATION, GET_MGMT_NODES)
                    .build(),
            )
            .value(())
            .build()
    }
}

/// If the request was successful then the statusCode MUST be 200 (OK) and the body of the message
/// MUST consist of an amqp-value section containing a list of addresses of other Management Nodes
/// known by this Management Node (each element of the list thus being a string). If no other
/// Management Nodes are known then the amqp-value section MUST contain a list of zero elements.
pub struct GetMgmtNodesResponse {
    pub addresses: Vec<String>,
}

impl GetMgmtNodesResponse {
    pub const STATUS_CODE: u16 = 200;
}

impl MessageDeserializer<Vec<String>> for GetMgmtNodesResponse {
    type Error = Error;

    fn from_message(message: Message<Vec<String>>) -> Result<Self> {
        match message.body {
            Body::Value(AmqpValue(addresses)) => Ok(Self { addresses }),
            _ => Err(Error::DecodeError),
        }
    }
}
