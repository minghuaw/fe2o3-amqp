use dotenv::dotenv;
use fe2o3_amqp::Sendable;
use fe2o3_amqp::types::messaging::Batch;
use fe2o3_amqp::types::messaging::Message;
use fe2o3_amqp::types::messaging::message::__private::Serializable;
use fe2o3_amqp::types::primitives::Binary;
use serde_amqp::to_vec;
use std::env;

use fe2o3_amqp::sasl_profile::SaslProfile;
use fe2o3_amqp::Connection;
use fe2o3_amqp::Sender;
use fe2o3_amqp::Session;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let hostname = env::var("HOST_NAME").unwrap();
    let port = 5671;
    let sa_key_name = env::var("SHARED_ACCESS_KEY_NAME").unwrap();
    let sa_key_value = env::var("SHARED_ACCESS_KEY_VALUE").unwrap();
    let queue_name = "q1";

    let url = format!("amqps://{}:{}", hostname, port);
    let mut connection = Connection::builder()
        .container_id("rust-connection-1")
        .alt_tls_establishment(true) // ServiceBus uses alternative TLS establishment
        .sasl_profile(SaslProfile::Plain {
            username: sa_key_name,
            password: sa_key_value,
        })
        .open(&url[..])
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", queue_name)
        .await
        .unwrap();

    // All of the Microsoft AMQP clients represent the event body as an uninterpreted bag of bytes.
    let data = Binary::from("hello world");
    let message = Message::from(data);
    let item1 = Binary::from(to_vec(&Serializable(message)).unwrap());
    let data2 = Binary::from("hello world 2");
    let message2 = Message::from(data2);
    let item2 = Binary::from(to_vec(&Serializable(message2)).unwrap());
    let message = Message::builder()
        .data_batch(Batch::new(vec![item1, item2]))
        .build();

    // let outcome = sender.send(message).await.unwrap();
    
    let sendable = Sendable::builder()
        .message(message)
        .message_format(2147563264)
        .build();
    let fut = sender.send_batchable_ref(&sendable).await.unwrap();
    let outcome = fut.await.unwrap();
    
    outcome.accepted_or_else(|outcome| outcome).unwrap();

    sender.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
