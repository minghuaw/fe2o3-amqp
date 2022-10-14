//! This example assumes you have an ActiveMQ instant that supports AMQP 1.0
//! running on your localhost

use fe2o3_amqp::{connection::Connection, session::Session, Receiver, Sender};

#[tokio::main]
async fn main() {
    let mut connection = Connection::open("connection-1", "amqp://guest:guest@localhost:5672")
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();
    let mut receiver = Receiver::attach(&mut session, "rust-receiver-link-1", "q1")
        .await
        .unwrap();

    let message = "hello AMQP";
    let outcome = sender.send(message).await.unwrap();
    outcome.accepted_or_else(|outcome| outcome).unwrap();
    println!("Sent: {:?}", message);

    let delivery = receiver.recv::<String>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();
    println!("Received: {:?}", delivery.body());

    sender.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
