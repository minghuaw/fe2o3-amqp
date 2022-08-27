pub struct GetMgmtNodesRequest {

}

pub struct GetMgmtNodesResponse {
    addresses: Vec<String>,
}

impl GetMgmtNodesResponse {
    const STATUS_CODE: u16 = 200;
}