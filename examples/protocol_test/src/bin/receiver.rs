use fe2o3_amqp::{connection::Connection, link::Receiver, session::Session};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tracing::info!("Starting connection");
    let mut connection = Connection::open(
        "connection-1",                      // container id
        "amqp://guest:guest@localhost:5672", // url
    )
    .await
    .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    // Create a receiver
    let mut receiver = Receiver::attach(
        &mut session,
        "rust-receiver-link-1", // link name
        "q1",                   // source address
    )
    .await
    .unwrap();

    // Receive the message from the broker
    let delivery = receiver.recv::<String>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();

    // sender.detach().await.unwrap(); // Detach sender with closing Detach performatives
    receiver.close().await.unwrap(); // Detach receiver with closing Detach performatives
    session.end().await.unwrap(); // End the session
    connection.close().await.unwrap(); // Close the connection
}
