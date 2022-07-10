use fe2o3_amqp::{Connection, Sender, Session};

#[tokio::main]
async fn main() {
    let mut connection =
        Connection::open("example-connection", "amqps://guest:guest@localhost:5672")
            .await
            .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();

    let outcome = sender.send("hello AMQP").await.unwrap();
    outcome.accepted_or_else(|outcome| outcome).unwrap();

    sender.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
