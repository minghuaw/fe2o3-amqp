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

#[tokio::main]
async fn main() {
    println!("Starting connection");

    let mut connection = Connection::builder()
        .container_id("fe2o3-amqp")
        .max_frame_size(1000)
        .channel_max(9)
        .idle_time_out(50_000 as u32)
        // .open("amqp://localhost:5672")
        .open("amqp://guest:guest@localhost:5672")
        .await
        .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    // let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
    //     .await
    //     .unwrap();

    let mut sender = Sender::builder()
        .name("rust-sender-link-1")
        .target("q1")
        .sender_settle_mode(SenderSettleMode::Mixed)
        .attach(&mut session)
        .await
        .unwrap();

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

    tokio::time::sleep(Duration::from_millis(500)).await;

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
    connection.close().await.unwrap();
}
