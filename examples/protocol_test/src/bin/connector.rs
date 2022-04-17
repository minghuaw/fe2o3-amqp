use tracing_subscriber::FmtSubscriber;
use tracing::{Level, instrument};
use fe2o3_amqp::{Connection, Session};

const BASE_ADDR: &str = "localhost:5671";

#[instrument]
async fn client_main() {
    let url = format!("amqp://{}", BASE_ADDR);
    let mut connection = Connection::open("connection-1", &url[..]).await.unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut session2 = Session::begin(&mut connection).await.unwrap();

    session2.end().await.unwrap();
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
