use fe2o3_amqp::{
    sasl_profile::SaslScramSha256, types::primitives::Value, Connection, Receiver, Sender,
    Session,
};

#[tokio::main]
async fn main() {
    let mut connection = Connection::builder()
        .container_id("connection-1")
        .sasl_profile(SaslScramSha256::new("guest", "guest"))
        .open("amqp://localhost:5672")
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();
    sender
        .send("hello AMQP")
        .await
        .unwrap()
        .accepted_or_else(|outcome| outcome)
        .unwrap();

    let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
        .await
        .unwrap();
    let delivery = receiver.recv::<Value>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();

    sender.close().await.unwrap();
    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
