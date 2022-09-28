use anyhow::{anyhow, Result};
use fe2o3_amqp::{connection::ConnectionHandle, types::primitives::Array, Session};
use fe2o3_amqp_management::{
    client::MgmtClient,
    operations::{ReadRequest, ReadResponse},
    response::Response,
};

pub async fn get_event_hub_partitions(
    connection: &mut ConnectionHandle<()>,
    event_hub_name: &str,
) -> Result<Vec<String>> {
    let mut session = Session::begin(connection).await?;
    let mut mgmt_client = MgmtClient::attach(&mut session, "mgmt_client_node").await?;

    let request = ReadRequest::name(event_hub_name);
    mgmt_client
        .send_request(request, "com.microsoft:eventhub", None)
        .await?
        .accepted_or(anyhow!("Request is not accepted"))?;

    let mut response: Response<ReadResponse> = mgmt_client.recv_response().await?;

    mgmt_client.close().await?;
    session.end().await?;

    let partition_value = response
        .operation
        .entity_attributes
        .remove("partition_ids")
        .ok_or(anyhow!("partition_ids not found"))?;
    let partitions: Array<String> = partition_value
        .try_into()
        .map_err(|val| anyhow!("Invalid partitions value: {:?}", val))?;
    Ok(partitions.into_inner())
}
