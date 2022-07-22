use fe2o3_amqp::{
    connection::Connection, link::Receiver, session::Session, types::primitives::Value, Delivery,
};

#[tokio::main]
async fn main() {
    let mut connection = Connection::open("connection-1", "amqp://localhost:5672")
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut receiver = Receiver::builder()
        .name("rust-receiver-link-1")
        .source("q1")
        .auto_accept(true) // default is `false`
        .attach(&mut session)
        .await
        .unwrap();

    // The delivery will be automatically accepted
    let delivery: Delivery<Value> = receiver.recv().await.unwrap();

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
