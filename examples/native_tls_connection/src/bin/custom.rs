use fe2o3_amqp::{Connection, Session, Sender};

#[tokio::main]
async fn main() {
    let connector = native_tls::TlsConnector::new().unwrap();
    let connector = tokio_native_tls::TlsConnector::from(connector);

    let mut connection = Connection::builder()
        .container_id("connection-1")
        .native_tls_connector(connector)
        .open("amqps://guest:guest@localhost:5671")
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
