
use fe2o3_amqp::{Connection, Session, types::{messaging::Source, primitives::Value}, Receiver};
use fe2o3_amqp_ext::filters::SelectorFilter;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    tracing::info!("Starting connection");

    // let address = "amqp://guest:guest@localhost:5672";
    let address = "amqp://localhost:5672";
    let mut connection = Connection::open("connection-1", address).await.unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();

    let source = Source::builder()
        .address("q1")
        .add_to_filter("f1", SelectorFilter(String::from("sn = 100")))
        .build();
    let mut receiver = Receiver::builder()
        .name("rust-receiver-link-1")
        .source(source)
        .auto_accept(false)
        .attach(&mut session)
        .await
        .unwrap();

    tracing::info!("Receiver attached");

    let delivery = receiver.recv::<Value>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();
    tracing::info!("{:?}", delivery);

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}