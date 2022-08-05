
use fe2o3_amqp::{Connection, Session};
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

    

    session.end().await.unwrap();
    connection.close().await.unwrap();
}