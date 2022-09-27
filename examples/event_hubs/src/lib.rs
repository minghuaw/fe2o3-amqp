use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use fe2o3_amqp::{
    connection::ConnectionHandle,
    types::{
        messaging::{ApplicationProperties, Message, Properties},
        primitives::Value,
    },
    Delivery, Receiver, Sender, Session,
};

pub async fn get_event_hub_partitions(
    connection: &mut ConnectionHandle<()>,
    event_hub_name: &str,
) -> Result<Vec<String>> {
    let mut session = Session::begin(connection).await?;

    // create a pair of links for request/response
    let client_node = "client-temp-node";
    let mut sender = Sender::attach(&mut session, "mgmt-sender", "$management").await?;
    let mut receiver = Receiver::builder()
        .name("mgmt-receiver")
        .source("$management")
        .target(client_node)
        .attach(&mut session)
        .await?;

    let request = Message::builder()
        .properties(
            Properties::builder()
                .message_id(String::from("request1"))
                .reply_to(client_node)
                .build(),
        )
        .application_properties(
            ApplicationProperties::builder()
                .insert("operation", "READ")
                .insert("name", event_hub_name)
                .insert("type", "com.microsoft:eventhub")
                .build(),
        )
        .value(())
        .build();
    sender
        .send(request)
        .await?
        .accepted_or(anyhow!("request not accepted"))?;

    let response: Delivery<BTreeMap<String, Value>> = receiver.recv().await?;
    receiver.accept(&response).await?;

    let mut response = response.try_into_value()?;
    let partitions = match response.remove("partition_ids") {
        Some(Value::Array(array)) => array,
        _ => return Err(anyhow!("partition_ids not found")),
    };
    let partitions: Vec<String> = partitions
        .into_inner()
        .into_iter()
        .map(|el| el.try_into())
        .collect::<std::result::Result<Vec<String>, Value>>()
        .map_err(|val| anyhow!("Expect string found {:?}", val))?;

    sender.close().await?;
    receiver.close().await?;
    session.close().await?;
    Ok(partitions)
}
