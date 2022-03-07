use std::time::Duration;

// use fe2o3_amqp::transport::connection::Connection;
// use fe2o3_amqp::transport::session::Session;

use fe2o3_amqp::connection::Connection;
use fe2o3_amqp::link::delivery::Sendable;
use fe2o3_amqp::link::Sender;
use fe2o3_amqp::session::Session;
use fe2o3_amqp::types::definitions::SenderSettleMode;
use fe2o3_amqp::types::messaging::message::BodySection;
use fe2o3_amqp::types::messaging::Message;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    // let (tx, mut rx) = mpsc::channel(1);
    // rx.close();
    // let result = tx.send(()).await;
    // println!("{:?}", result);

    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // .with_max_level(Level::DEBUG)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut connection = Connection::open("connection-1", "amqp://guest:guest@localhost:5672")
        .await
        .unwrap();
    // let mut connection = Connection::builder()
    //     .container_id("connection-1")
    //     .max_frame_size(1000)
    //     .channel_max(9)
    //     .idle_time_out(50_000 as u32)
    //     // .open("amqp://localhost:5672")
    //     .open("amqp://guest:guest@localhost:5672")
    //     .await
    //     .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

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

    let body = BodySection::from("hello body_section");
    // let message = Message::from("hello");
    let message = Message::from(body);
    let message = Sendable::from(message);
    // let message = Sendable::builder()
    //     .message("hello world")
    //     .settled(true)
    //     .build();

    sender.send(message).await.unwrap();

    sender.send("hello").await.unwrap();

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

    sender.close().await.unwrap();

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
