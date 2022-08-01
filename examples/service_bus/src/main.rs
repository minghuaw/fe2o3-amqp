use dotenv::dotenv;
use fe2o3_amqp::Receiver;
use std::env;
use std::sync::Arc;

use fe2o3_amqp::sasl_profile::SaslProfile;
use fe2o3_amqp::Connection;
use fe2o3_amqp::Sender;
use fe2o3_amqp::Session;
use rustls::OwnedTrustAnchor;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let hostname = env::var("HOST").unwrap();
    let port = 5671;
    let username = env::var("USER").unwrap();
    let password = env::var("PASSWORD").unwrap();
    let queue_name = "q1";

    let mut root_store = rustls::RootCertStore::empty();
    root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));
    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let stream = TcpStream::connect((&hostname[..], port)).await.unwrap();
    let domain = hostname.as_str().try_into().unwrap();
    let connector = TlsConnector::from(Arc::new(config));
    let tls_stream = connector.connect(domain, stream).await.unwrap();

    let mut connection = Connection::builder()
        .container_id("rust-connection-1")
        .hostname(&hostname[..])
        .sasl_profile(SaslProfile::Plain { username, password })
        .open_with_stream(tls_stream)
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();

    // let mut sender = Sender::attach(&mut session, "rust-sender-link-1", queue_name)
    //     .await
    //     .unwrap();

    // let outcome = sender.send("hello AMQP").await.unwrap();
    // outcome.accepted_or_else(|outcome| outcome).unwrap();
    // sender.close().await.unwrap();

    let mut receiver = Receiver::attach(&mut session, "rust-receiver-link-1", queue_name)
        .await
        .unwrap();
    let delivery = receiver.recv::<String>().await.unwrap();
    println!("Received: {:?}", delivery);
    receiver.accept(&delivery).await.unwrap();
    receiver.close().await.unwrap();

    session.end().await.unwrap();
    connection.close().await.unwrap();
}
