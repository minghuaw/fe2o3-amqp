use std::collections::BTreeMap;
use std::time::Duration;

use fe2o3_amqp::sasl_profile::SaslProfile;
use fe2o3_amqp::types::definitions::ReceiverSettleMode;
use fe2o3_amqp::types::definitions::SenderSettleMode;
use fe2o3_amqp::types::messaging::ApplicationProperties;
use fe2o3_amqp::types::messaging::MessageId;
use fe2o3_amqp::types::messaging::Properties;
use fe2o3_amqp::types::messaging::message::Body;
use fe2o3_amqp::types::messaging::Message;
use fe2o3_amqp::Connection;
use fe2o3_amqp::Sendable;
use fe2o3_amqp::Sender;
use fe2o3_amqp::Session;
use fe2o3_amqp::types::primitives::SimpleValue;
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

    // let mut sender = Sender::builder()
    //     .name("rust-sender-link-1")
    //     .target("q1")
    //     .sender_settle_mode(SenderSettleMode::Settled)
    //     .receiver_settle_mode(ReceiverSettleMode::Second)
    //     .attach(&mut session)
    //     .await
    //     .unwrap();

    // let body = Body::from(());
    // let message = Message::from("hello");
    // let props = Properties::builder()
    //         .message_id(MessageId::ULong(1))
    //         .build();
    // let mut application_properties = BTreeMap::new();
    // application_properties.insert(String::from("sn"), SimpleValue::UInt(100));
    // let message = Message::builder()
    //         .properties(props)
    //         .application_properties(ApplicationProperties(application_properties))
    //         .value("hello AMQP")
    //         .build();

    let message = Message::builder()
        .properties(Properties::builder().message_id(1).build())
        .application_properties(ApplicationProperties::builder().insert("sn", 10).build())
        .value("hello AMQP")
        .build();

    let message = Sendable::from(message);
    // let message = Sendable::builder()
    //     .message("hello world")
    //     .settled(true)
    //     .build();
    
    // let handle = tokio::spawn(async move {
    //     sender.send(message).await.unwrap();
    
    //     sender.send("world").await.unwrap();
    //     sender.detach().await.unwrap();
    // });

    // handle.await;

    // let outcome = sender.send(message).await.unwrap();
    let fut = sender.send_batchable(message).await.unwrap();
    let outcome = fut.await.unwrap();

    // Checks the outcome of delivery
    if outcome.is_accepted() {
        tracing::info!("Outcome: {:?}", outcome)
    } else {
        tracing::error!("Outcome: {:?}", outcome)
    }

    // let detached = sender.detach().await.unwrap();
    // let mut sender = detached.resume().await.unwrap();

    // sender.send("hello again").await.unwrap();
    
    sender.close().await.unwrap();

    session.end().await.unwrap();
    // let result = session.on_end().await;
    // println!("{:?}", result);
    connection.close().await.unwrap();
    // let result = connection.on_close().await;
    // println!("{:?}", result);
}
