//! This example assumes you have an ActiveMQ instant that supports AMQP 1.0
//! running on your localhost
//! 
//! `ActiveMQ` uses alternative TLS establishment (ie. establish TLS without 
//! exchanging ['A', 'M', 'Q', 'P', '2', '1', '0', '0'] header). 
//! 
//! - The `"rustls"` example shows 
//! the more complicated way to perform alternative TLS establishment - manually/explicitly establish
//! a `tls_stream` and then pass it to `Connection`. 
//! 
//! - The `"native_tls"` example will show how to 
//! use a config to ask the `Connection` to do this implicitly. The user should also check the 
//! `service_bus` example to see how to establish alternative TLS connection implicitly.
//! 
//! Please note that you may need to explicitly set you `ActiveMQ` to use TLSv1.2 or higher
//! in the xml configuration file.
//! 
//! ```xml
//! <transportConnector name="amqp+ssl" uri="amqp+ssl://0.0.0.0:5671?transport.enabledProtocols=TLSv1.2"/>
//! ```

use std::sync::Arc;

use fe2o3_amqp::Connection;
use fe2o3_amqp::Sender;
use fe2o3_amqp::Session;
use fe2o3_amqp::sasl_profile::SaslProfile;
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
        .sasl_profile(SaslProfile::Plain {
            username: "guest".into(),
            password: "guest".into(),
        })
        .open_with_stream(tls_stream)
        .await
        .unwrap();

    // // The commented code below is equivalent to the codes above with "rustls" feature enabled
    // let addr = "amqps://guest:guest@localhost:5671";
    // let mut connection = Connection::builder()
    //     .container_id("rust-connection-1")
    //     .alt_tls_establishment(true) // uses alternative TLS establishement
    //     .open(addr)
    //     .await
    //     .unwrap();

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
