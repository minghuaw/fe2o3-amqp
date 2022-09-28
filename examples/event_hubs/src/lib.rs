use anyhow::{anyhow, Result};
use fe2o3_amqp::{connection::ConnectionHandle, types::primitives::Array, Session};
use fe2o3_amqp_management::{client::MgmtClient, operations::ReadRequest};

pub async fn get_event_hub_partitions(
    connection: &mut ConnectionHandle<()>,
    event_hub_name: &str,
) -> Result<Vec<String>> {
    let mut session = Session::begin(connection).await?;
    let mut mgmt_client = MgmtClient::attach(&mut session, "mgmt_client_node").await?;

    let request = ReadRequest::name(event_hub_name);
    let mut response = mgmt_client
        .read(request, "com.microsoft:eventhub", None)
        .await?;

    mgmt_client.close().await?;
    session.end().await?;

    let partition_value = response
        .entity_attributes
        .remove("partition_ids")
        .ok_or(anyhow!("partition_ids not found"))?;
    let partitions: Array<String> = partition_value
        .try_into()
        .map_err(|val| anyhow!("Invalid partitions value: {:?}", val))?;
    Ok(partitions.into_inner())
}
