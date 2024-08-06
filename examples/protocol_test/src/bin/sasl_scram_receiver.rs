use fe2o3_amqp::{sasl_profile::scram::SaslScramSha256, Connection, Receiver, Session};

#[tokio::main]
async fn main() {
    let mut connection = Connection::builder()
        .container_id("connection-1")
        .sasl_profile(SaslScramSha256::new("guest", "guest"))
        // .sasl_profile(SaslProfile::Plain {
        //     username: "guest".to_string(),
        //     password: "guest".to_string(),
        // })
        .open("amqp://localhost:5675")
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
        .await
        .unwrap();

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
