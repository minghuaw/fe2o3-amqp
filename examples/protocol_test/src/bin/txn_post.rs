use fe2o3_amqp::{
    sasl_profile::SaslProfile,
    transaction::{Controller, Transaction, TransactionDischarge},
    Connection, Sender, Session,
};
use tokio::net::TcpStream;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // .with_max_level(Level::DEBUG)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let addr = "localhost:5671";
    let domain = "localhost";
    let stream = TcpStream::connect(addr).await.unwrap();
    let connector = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let connector = tokio_native_tls::TlsConnector::from(connector);
    let tls_stream = connector.connect(domain, stream).await.unwrap();

    let mut connection = Connection::builder()
        .container_id("connection-1")
        .scheme("amqp")
        .sasl_profile(SaslProfile::Plain {
            username: "guest".into(),
            password: "guest".into(),
        })
        .open_with_stream(tls_stream)
        .await
        .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();
    let mut controller = Controller::attach(&mut session, "controller-1")
        .await
        .unwrap();

    // Commit
    let txn1 = Transaction::declare(&mut controller, None).await.unwrap();
    txn1.post(&mut sender, "hello").await.unwrap();
    txn1.post(&mut sender, "world").await.unwrap();
    txn1.commit().await.unwrap();

    // Rollback
    let txn2 = Transaction::declare(&mut controller, None).await.unwrap();
    txn2.post(&mut sender, "foo").await.unwrap();
    txn2.rollback().await.unwrap();

    controller.close().await.unwrap();
    sender.close().await.unwrap();
    session.close().await.unwrap();
    connection.close().await.unwrap();
}
