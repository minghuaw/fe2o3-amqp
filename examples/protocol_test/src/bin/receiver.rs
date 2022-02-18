use fe2o3_amqp::{connection::Connection, session::Session, link::Receiver, types::primitives::Value};

#[tokio::main]
async fn main() {
    println!("Starting connection");

    let mut connection = Connection::builder()
        .container_id("fe2o3-amqp")
        .max_frame_size(1000)
        .channel_max(9)
        .idle_time_out(50_000 as u32)
        .open("amqp://127.0.0.1:5672")
        .await
        .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    let mut receiver = Receiver::builder()
        .name("rust-receiver-link-1")
        .source("q1")
        .attach(&mut session)
        .await
        .unwrap();
    
    println!("Receiver attached");
    // tokio::time::sleep(Duration::from_millis(500)).await;

    let delivery = receiver.recv::<Value>().await.unwrap();
    println!("<<< Message >>> {:?}", delivery);
    receiver.accept(&delivery).await.unwrap();

    let delivery = receiver.recv::<Value>().await.unwrap();
    println!("<<< Message >>> {:?}", delivery);
    receiver.accept(&delivery).await.unwrap();

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}