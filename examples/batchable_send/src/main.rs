use fe2o3_amqp::{connection::Connection, session::Session, Sender};

#[tokio::main]
async fn main() {
    let mut connection = Connection::open("connection-1", "amqp://localhost:5672")
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();

    let fut1 = sender.send_batchable("hello AMQP").await.unwrap();
    let fut2 = sender.send_batchable("hello world").await.unwrap();

    let outcome1 = fut1.await.unwrap();
    outcome1.accepted_or_else(|outcome| outcome).unwrap();

    let outcome2 = fut2.await.unwrap();
    outcome2.accepted_or_else(|outcome| outcome).unwrap();

    sender.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
