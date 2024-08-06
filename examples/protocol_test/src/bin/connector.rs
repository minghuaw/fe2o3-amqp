use std::time::Duration;

use fe2o3_amqp::{
    types::{
        definitions::{ReceiverSettleMode, SenderSettleMode},
        messaging::Body,
        primitives::Value,
    },
    Connection, Delivery, Receiver, Sender, Session,
};
use tracing::{instrument, Level};
use tracing_subscriber::FmtSubscriber;

const SASL_PLAIN: &str = "";
// const SASL_PLAIN: &str = "guest:guest";
const BASE_ADDR: &str = "localhost:5672";

#[instrument]
async fn client_main() {
    let url = format!("amqp://{}@{}", SASL_PLAIN, BASE_ADDR);
    let mut connection = Connection::open("connection-1", &url[..]).await.unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();
    // let mut session2 = Session::begin(&mut connection).await.unwrap();

    // let mut sender = Sender::attach(&mut session, "sender-1", "q1")
    //     .await
    //     .unwrap();
    let mut sender = Sender::builder()
        .name("sender-1")
        .target("q1")
        .sender_settle_mode(SenderSettleMode::Mixed)
        .receiver_settle_mode(ReceiverSettleMode::First)
        .attach(&mut session)
        .await
        .unwrap();

    sender.send("hello").await.unwrap();

    let mut receiver = Receiver::attach(&mut session, "receiver-1", "q1")
        .await
        .unwrap();
    let delivery: Delivery<Body<Value>> = receiver.recv().await.unwrap();
    tracing::info!(message = ?delivery.message());
    receiver.accept(&delivery).await.unwrap();

    // session2.end().await.unwrap();

    sender.close().await.unwrap();
    receiver.close().await.unwrap();
    // Add a small sleep to avoid spuriously ending the session
    // before the Detach are received
    tokio::time::sleep(Duration::from_millis(500)).await;
    session.end().await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    client_main().await;
}
