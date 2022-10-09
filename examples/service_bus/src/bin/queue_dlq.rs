//! Dead letter queue for Service Bus Queue

use dotenv::dotenv;
use fe2o3_amqp::connection::ConnectionHandle;
use fe2o3_amqp::transaction::OwnedTransaction;
use fe2o3_amqp::transaction::TransactionDischarge;
use fe2o3_amqp::transaction::TransactionalRetirement;
use fe2o3_amqp::types::messaging::Body;
use fe2o3_amqp::types::messaging::Message;
use fe2o3_amqp::types::messaging::Properties;
use fe2o3_amqp::types::primitives::Binary;
use fe2o3_amqp::types::primitives::Value;
use fe2o3_amqp::Receiver;
use fe2o3_amqp::Sender;
use std::env;

use fe2o3_amqp::sasl_profile::SaslProfile;
use fe2o3_amqp::Connection;
use fe2o3_amqp::Session;

async fn create_dlq_message(connection: &mut ConnectionHandle<()>, queue_name: &str) {
    let mut session = Session::begin(connection).await.unwrap();

    // Send a message
    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", queue_name)
        .await
        .unwrap();

    // All of the Microsoft AMQP clients represent the event body as an uninterpreted bag of bytes.
    let data = "hello AMQP from rust".as_bytes().to_vec();
    let message = Message::builder()
        .properties(Properties::builder().message_id(1).build())
        .data(Binary::from(data))
        .build();

    let outcome = sender.send(message).await.unwrap();
    outcome.accepted_or_else(|outcome| outcome).unwrap();
    sender.close().await.unwrap();

    // Reject the message
    // "... it can reject the message, which moves it into the dead-letter queue ..."
    let mut receiver = Receiver::attach(&mut session, "rust-receiver-link-1", queue_name)
        .await
        .unwrap();
    let delivery = receiver.recv::<Body<Value>>().await.unwrap();
    receiver.reject(&delivery, None).await.unwrap();
    receiver.close().await.unwrap();

    session.end().await.unwrap();
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let hostname = env::var("HOST_NAME").unwrap();
    let port = 5671;
    let sa_key_name = env::var("SHARED_ACCESS_KEY_NAME").unwrap();
    let sa_key_value = env::var("SHARED_ACCESS_KEY_VALUE").unwrap();
    let queue_name = env::var("QUEUE_NAME").unwrap();

    let url = format!("amqps://{}:{}", hostname, port);
    let mut connection = Connection::builder()
        .container_id("rust-sender-connection-1")
        .alt_tls_establishment(true) // ServiceBus uses alternative TLS establishement
        .sasl_profile(SaslProfile::Plain {
            username: sa_key_name,
            password: sa_key_value,
        })
        .open(&url[..])
        .await
        .unwrap();

    // Create a DLQ message by rejecting it
    create_dlq_message(&mut connection, &queue_name[..]).await;

    let mut session = Session::begin(&mut connection).await.unwrap();

    // DLQ address
    let addr = format!("{}/$deadletterqueue", queue_name);
    let mut receiver = Receiver::attach(&mut session, "rust-receiver-link-1", addr)
        .await
        .unwrap();

    // All of the Microsoft AMQP clients represent the event body as an uninterpreted bag of bytes.
    let delivery = receiver.recv::<Body<Value>>().await.unwrap();
    println!("Received from DLQ: {:?}", delivery);

    // The Azure ServiceBus SDK disposes the DLQ message in a txn
    let mut txn = OwnedTransaction::declare(&mut session, "complete_dlq_message_controller", None)
        .await
        .unwrap();
    txn.accept(&mut receiver, &delivery).await.unwrap();
    txn.commit().await.unwrap();

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
