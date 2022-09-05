use std::env;

use dotenv::dotenv;
use fe2o3_amqp::{
    sasl_profile::SaslProfile,
    types::{
        messaging::{Message, Properties},
        primitives::Binary,
    },
    Connection, Sender, Session,
};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let hostname = env::var("HOST_NAME").unwrap();
    let port = 5671;
    let sa_key_name = env::var("SHARED_ACCESS_KEY_NAME").unwrap();
    let sa_key_value = env::var("SHARED_ACCESS_KEY_VALUE").unwrap();
    let topic_name = env::var("TOPIC_NAME").unwrap();

    let url = format!("amqps://{}:{}", hostname, port);
    let mut connection = Connection::builder()
        .container_id("rust-sender-connection-1")
        .alt_tls_establishment(true) // ServiceBus uses alternative TLS establishement
        .hostname(&hostname[..])
        .sasl_profile(SaslProfile::Plain {
            username: sa_key_name,
            password: sa_key_value,
        })
        .open(&url[..])
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();

    let mut sender = Sender::attach(&mut session, "rust-topic-sender", topic_name)
        .await
        .unwrap();

    for i in 0..3 {
        let message_id = format!("topic_test_{}", i);
        // All of the Microsoft AMQP clients represent the event body as an uninterpreted bag of bytes.
        let data = format!("Message {}", i).into_bytes();
        let message = Message::builder()
            .properties(Properties::builder().message_id(message_id).build())
            .data(Binary::from(data))
            .build();
        let outcome = sender.send(message).await.unwrap();
        outcome.accepted_or_else(|outcome| outcome).unwrap();
    }

    sender.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
