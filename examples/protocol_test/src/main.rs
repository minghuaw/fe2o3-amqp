use std::time::Duration;

// use fe2o3_amqp::transport::connection::Connection;
// use fe2o3_amqp::transport::session::Session;

use fe2o3_amqp::connection::Connection;
use fe2o3_amqp::link::{Receiver, Sender};
use fe2o3_amqp::session::Session;
use fe2o3_amqp::types::definitions::SenderSettleMode;

#[tokio::main]
async fn main() {
    println!("Starting connection");

    let mut connection = Connection::builder()
        .container_id("fe2o3-amqp")
        .hostname("127.0.0.1")
        .max_frame_size(1000)
        .channel_max(9)
        .idle_time_out(50_000 as u32)
        .open("amqp://127.0.0.1:5674")
        .await
        .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();

    // let mut sender = Sender::builder()
    //     .name("rust-sender-link-1")
    //     .target("q1")
    //     .sender_settle_mode(SenderSettleMode::Settled)
    //     .attach(&mut session)
    //     .await
    //     .unwrap();

    // sender.send("hello amqp").await.unwrap();

    // // sender.close().await.unwrap();
    // if let Err(err) = sender.detach().await {
    //     println!("+++++++++ {:?}", err)
    // }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // let mut sender = Sender::attach(&mut session, "sender-link-2", "q1")
    //     .await
    //     .unwrap();

    // sender.send("HELLO AMQP").await.unwrap();

    let fut = sender.send_batchable("HELLO AMQP").await.unwrap();

    let result = fut.await;
    println!("fut {:?}", result);

    sender.close().await.unwrap();

    let receiver = Receiver::attach(&mut session, "rust-receiver-link-1", "q1")
        .await
        .unwrap();
    let result = receiver.detach().await;
    println!("{:?}", result);
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
