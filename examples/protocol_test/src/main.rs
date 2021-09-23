use std::time::Duration;

use fe2o3_engine::transport::connection::Connection;

#[tokio::main]
async fn main() {
    println!("Starting connection");

    let mut connection = Connection::builder()
        .container_id("1234")
        .hostname("127.0.0.1")
        .max_frame_size(1000)
        .channel_max(9)
        .idle_time_out(50_000 as u32)
        .open("amqp://127.0.0.1:5674")
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;

    connection.close().await.unwrap();
}
