use fe2o3_amqp::{
    connection::Connection,
    session::Session,
    Sender,
};

#[tokio::main]
async fn main() {
    let mut connection = Connection::open("connection-1", "amqp://localhost:5672").await.unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();

    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();

    // Send simple hello AMQP
    sender.send("hello AMQP").await.unwrap()
        .accepted_or_else(|outcome| outcome).unwrap();

    session.end().await.unwrap();
    connection.close().await.unwrap();
}
