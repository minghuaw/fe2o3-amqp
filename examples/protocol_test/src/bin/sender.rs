use std::collections::BTreeMap;
use std::time::Duration;

use fe2o3_amqp::sasl_profile::SaslProfile;
use fe2o3_amqp::types::definitions::ReceiverSettleMode;
use fe2o3_amqp::types::definitions::SenderSettleMode;
use fe2o3_amqp::types::messaging::AmqpSequence;
use fe2o3_amqp::types::messaging::ApplicationProperties;
use fe2o3_amqp::types::messaging::Data;
use fe2o3_amqp::types::messaging::MessageId;
use fe2o3_amqp::types::messaging::Properties;
use fe2o3_amqp::types::messaging::message::Body;
use fe2o3_amqp::types::messaging::Message;
use fe2o3_amqp::Connection;
use fe2o3_amqp::Sendable;
use fe2o3_amqp::Sender;
use fe2o3_amqp::Session;
use fe2o3_amqp::types::primitives::Binary;
use fe2o3_amqp::types::primitives::SimpleValue;
use fe2o3_amqp::types::primitives::Value;
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

    // let addr = "localhost:5671";
    // let domain = "localhost";
    // let stream = TcpStream::connect(addr).await.unwrap();
    // let connector = native_tls::TlsConnector::builder()
    //     .danger_accept_invalid_certs(true)
    //     .build()
    //     .unwrap();
    // let connector = tokio_native_tls::TlsConnector::from(connector);
    // let tls_stream = connector.connect(domain, stream).await.unwrap();

    // let mut connection = Connection::builder()
    //     .container_id("connection-1")
    //     .scheme("amqp")
    //     .max_frame_size(1000)
    //     .channel_max(9)
    //     .idle_time_out(50_000 as u32)
    //     // .sasl_profile(SaslProfile::Plain {
    //     //     username: "guest".into(),
    //     //     password: "guest".into(),
    //     // })
    //     .open_with_stream(tls_stream)
    //     // .open("amqp://localhost:5672")
    //     // .open("amqp://guest:guest@localhost:5672")
    //     // .open("localhost:5672")
    //     .await
    //     .unwrap();

    let mut connection = Connection::open("connection-1", "amqp://localhost:5672")
        .await
        .unwrap();

    // let mut session = Session::begin(&mut connection).await.unwrap();
    let mut session = Session::builder()
        .handle_max(128)
        .begin(&mut connection)
        .await
        .unwrap();

    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();

    struct Foo {}

    impl From<Foo> for Message<Value> {
        fn from(_: Foo) -> Self {
            Message::builder().data(Binary::from("Foo")).build()
        }
    }

    // let outcome = sender.send(message).await.unwrap();
    let fut = sender.send_batchable(Foo {}).await.unwrap();
    let outcome = fut.await.unwrap();

    // Checks the outcome of delivery
    if outcome.is_accepted() {
        tracing::info!("Outcome: {:?}", outcome)
    } else {
        tracing::error!("Outcome: {:?}", outcome)
    }
    sender.close().await.unwrap();

    session.end().await.unwrap();
    // let result = session.on_end().await;
    // println!("{:?}", result);
    connection.close().await.unwrap();
    // let result = connection.on_close().await;
    // println!("{:?}", result);
}
