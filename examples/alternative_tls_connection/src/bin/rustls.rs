use std::sync::Arc;

use fe2o3_amqp::Connection;
use fe2o3_amqp::Sender;
use fe2o3_amqp::Session;
use rustls::pki_types::ServerName;
use rustls::ClientConfig;
use rustls::RootCertStore;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

#[tokio::main]
async fn main() {
    let addr = "localhost:5671";
    let domain = ServerName::try_from("localhost").unwrap();
    let stream = TcpStream::connect(addr).await.unwrap();

    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let tls_stream = connector.connect(domain, stream).await.unwrap();

    let mut connection = Connection::builder()
        .container_id("connection-1")
        .open_with_stream(tls_stream)
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
