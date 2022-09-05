use fe2o3_amqp::{
    connection::Connection, link::Receiver, session::Session, types::primitives::Value,
};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tracing::info!("Starting connection");

    let mut connection =
        Connection::open("receiver_connection", "amqp://guest:guest@localhost:5672")
            .await
            .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
        .await
        .unwrap();

    tracing::info!("Receiver attached");

    let delivery = receiver.recv::<Value>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();
    println!("{:?}", delivery);

    let delivery = receiver.recv::<Value>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();
    println!("{:?}", delivery.body());
    println!("{:?}", delivery.body().is_data());

    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
