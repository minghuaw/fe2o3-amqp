
use crate::error::Result;

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
pub struct GetMgmtNodesRequest {

}

pub struct GetMgmtNodesResponse {
    addresses: Vec<String>,
}

impl GetMgmtNodesResponse {
    const STATUS_CODE: u16 = 200;
}