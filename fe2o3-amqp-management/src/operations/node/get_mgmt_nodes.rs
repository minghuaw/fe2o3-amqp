use fe2o3_amqp_types::messaging::{ApplicationProperties, Message};

use crate::{
    error::Result,
    operations::{GET_MGMT_NODES, OPERATION},
    request::MessageSerializer,
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

pub struct GetMgmtNodesResponse {
    addresses: Vec<String>,
}

impl GetMgmtNodesResponse {
    const STATUS_CODE: u16 = 200;
}
