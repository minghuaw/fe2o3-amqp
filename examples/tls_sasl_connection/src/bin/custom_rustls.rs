use std::sync::Arc;

use fe2o3_amqp::{Connection, Sender, Session, sasl_profile::SaslProfile};
use rustls::{RootCertStore, OwnedTrustAnchor, ClientConfig};
use tokio_rustls::TlsConnector;

#[tokio::main]
async fn main() {
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));

    let mut connection = Connection::builder()
        .container_id("connection-1")
        .rustls_connector(connector)
        .sasl_profile(SaslProfile::Plain { username: String::from("guest"), password: String::from("guest") })
        .open("amqps://localhost:5671")
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
