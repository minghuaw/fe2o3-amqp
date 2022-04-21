use std::time::Duration;

use fe2o3_amqp::{Connection, Receiver, Sender, Session};
use tracing::{info, instrument, Level};
use tracing_subscriber::FmtSubscriber;

const BASE_ADDR: &str = "localhost:5672";

#[instrument]
async fn client_main() {
    let url = format!("amqp://{}", BASE_ADDR);
    let mut connection = Connection::open("connection-1", &url[..]).await.unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();
    // let mut session2 = Session::begin(&mut connection).await.unwrap();

    let sender = Sender::attach(&mut session, "sender-1", "q1").await.unwrap();
    let receiver = Receiver::attach(&mut session, "receiver-1", "q1").await.unwrap();

    // session2.end().await.unwrap();

    receiver.close().await.unwrap();
    sender.close().await.unwrap();
    // tokio::time::sleep(Duration::from_millis(1000)).await;
    info!(">>> link is closed");
    session.end().await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    client_main().await;
}
