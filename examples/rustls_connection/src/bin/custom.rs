use std::sync::Arc;

use fe2o3_amqp::{Connection, Session, Sender};
use rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

#[tokio::main]
async fn main() {
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));

    let mut connection = Connection::builder()
        .container_id("connection-1")
        .rustls_connector(connector)
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
