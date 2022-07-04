use std::time::Duration;

use fe2o3_amqp::sasl_profile::SaslProfile;
use fe2o3_amqp::types::messaging::message::Body;
use fe2o3_amqp::types::messaging::Message;
use fe2o3_amqp::Connection;
use fe2o3_amqp::Sendable;
use fe2o3_amqp::Sender;
use fe2o3_amqp::Session;
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

    // let mut connection = Connection::open("connection-1", "amqp://guest:guest@localhost:5671")
    //     .await
    //     .unwrap();
    let mut connection = Connection::builder()
        .container_id("connection-1")
        .scheme("amqp")
        .max_frame_size(1000)
        .channel_max(9)
        .idle_time_out(50_000 as u32)
        // .sasl_profile(SaslProfile::Plain {
        //     username: "guest".into(),
        //     password: "guest".into(),
        // })
        // .open_with_stream(tls_stream)
        .open("amqp://localhost:5672")
        // .open("amqp://guest:guest@localhost:5672")
        // .open("localhost:5672")
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
    //     .sender_settle_mode(SenderSettleMode::Mixed)
    //     .attach(&mut session)
    //     .await
    //     .unwrap();

    let body = Body::from("hello");
    // let message = Message::from("hello");
    let message = Message::from(body);
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

    let outcome = sender.send(message).await.unwrap();

    // Checks the outcome of delivery
    if outcome.is_accepted() {
        tracing::info!("Outcome: {:?}", outcome)
    } else {
        tracing::error!("Outcome: {:?}", outcome)
    }
    
    // sender.send("world").await.unwrap();
    sender.close().await.unwrap();

    // // sender.close().await.unwrap();
    // if let Err(err) = sender.detach().await {
    //     println!("+++++++++ {:?}", err)
    // }

    // let mut sender = Sender::attach(&mut session, "sender-link-2", "q1")
    //     .await
    //     .unwrap();

    // sender.send("HELLO AMQP").await.unwrap();

    // let fut = sender.send_batchable("HELLO AMQP").await.unwrap();

    // let result = fut.await;
    // println!("fut {:?}", result);



    // let receiver = Receiver::attach(&mut session, "rust-receiver-link-1", "q1")
    //     .await
    //     .unwrap();
    // let result = receiver.detach().await;
    // println!("{:?}", result);
    session.end().await.unwrap();
    // let result = session.on_end().await;
    // println!("{:?}", result);
    tokio::time::sleep(Duration::from_millis(500)).await;
    connection.close().await.unwrap();
    // let result = connection.on_close().await;
    // println!("{:?}", result);
}
