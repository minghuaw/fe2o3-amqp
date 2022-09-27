use std::env;

use dotenv::dotenv;
use event_hubs::get_event_hub_partitions;
use fe2o3_amqp::{
    sasl_profile::SaslProfile,
    types::{
        definitions::SECURE_PORT,
        messaging::{Message, Properties},
        primitives::Binary,
    },
    Connection, Sender, Session,
};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let hostname = env::var("HOST_NAME").unwrap();
    let port = SECURE_PORT;
    let sa_key_name = env::var("SHARED_ACCESS_KEY_NAME").unwrap();
    let sa_key_value = env::var("SHARED_ACCESS_KEY_VALUE").unwrap();
    let event_hub_name = env::var("EVENT_HUB_NAME").unwrap();

    let url = format!("amqps://{}:{}", hostname, port);
    let mut connection = Connection::builder()
        .container_id("rust-connection-1")
        .alt_tls_establishment(true) // EventHubs uses alternative TLS establishment
        .sasl_profile(SaslProfile::Plain {
            username: sa_key_name,
            password: sa_key_value,
        })
        .open(&url[..])
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();

    let partitions = get_event_hub_partitions(&mut connection, &event_hub_name)
        .await
        .unwrap();
    let partition = &partitions[0];

    let partition_address = format!("{}/Partitions/{}", event_hub_name, partition);
    let mut sender = Sender::attach(&mut session, "rust-simple-sender", partition_address)
        .await
        .unwrap();

    // Message will be randomly distributed to different partitions
    for i in 0..3 {
        // All of the Microsoft AMQP clients represent the event body as an uninterpreted bag of bytes.
        let data = format!("Message {}", i).into_bytes();
        let message = Message::builder()
            .properties(
                Properties::builder()
                    .group_id(String::from("send_to_event_hub"))
                    .build(),
            )
            .data(Binary::from(data))
            .build();
        let outcome = sender.send(message).await.unwrap();
        outcome.accepted_or_else(|outcome| outcome).unwrap();
    }

    sender.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
