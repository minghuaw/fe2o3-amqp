use fe2o3_amqp::{connection::Connection, session::Session, types::messaging::Target, Sender};

#[tokio::main]
async fn main() {
    let mut connection = Connection::open("connection-1", "amqp://localhost:5672")
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let sender = Sender::builder()
        .name("dynamic-sender")
        .target(Target::builder().dynamic(true).build())
        .attach(&mut session)
        .await
        .unwrap();

    sender.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
