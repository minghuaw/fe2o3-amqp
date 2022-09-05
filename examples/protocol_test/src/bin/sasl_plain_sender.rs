use fe2o3_amqp::types::messaging::Message;
use fe2o3_amqp::types::primitives::Binary;
use fe2o3_amqp::types::primitives::Value;
use fe2o3_amqp::Connection;
use fe2o3_amqp::Sender;
use fe2o3_amqp::Session;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut connection = Connection::open("connection-1", "amqp://guest:guest@localhost:5672")
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

    struct Foo {}

    impl From<Foo> for Message<Value> {
        fn from(_: Foo) -> Self {
            Message::builder().data(Binary::from("Foo")).build()
        }
    }

    // let outcome = sender.send(message).await.unwrap();
    let fut = sender.send_batchable(Foo {}).await.unwrap();
    let outcome = fut.await.unwrap();

    // Checks the outcome of delivery
    if outcome.is_accepted() {
        tracing::info!("Outcome: {:?}", outcome)
    } else {
        tracing::error!("Outcome: {:?}", outcome)
    }
    sender.close().await.unwrap();

    session.end().await.unwrap();
    connection.close().await.unwrap();
}
