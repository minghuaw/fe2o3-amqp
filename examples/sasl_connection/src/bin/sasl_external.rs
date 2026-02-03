use rustls::{ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use std::sync::Arc;
use fe2o3_amqp::{
    sasl_profile::SaslProfile, types::primitives::Value, Connection, Receiver, Sender,
    Session,
};

#[tokio::main]
async fn main() {

    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder()
                              .with_root_certificates(root_store)
                              .with_no_client_auth();
    let tls_connector = TlsConnector::from(Arc::new(config));

    let stream = TcpStream::connect("localhost:5671").await.unwrap();

    let mut connection = Connection::builder()
        .container_id("connection-1")
        .tls_connector(tls_connector)
        .sasl_profile(SaslProfile::External)
        .domain("localhost")
        .hostname("localhost:5671")
        .scheme("amqps")
        .alt_tls_establishment(true)
        .open_with_stream(stream).await.unwrap();
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
