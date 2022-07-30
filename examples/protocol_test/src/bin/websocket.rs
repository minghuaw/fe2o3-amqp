use fe2o3_amqp::{
    connection::Connection, link::Sender, session::Session, types::primitives::Value, Delivery, sasl_profile::SaslProfile,
};
use fe2o3_amqp_ws::WebSocketStream;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let (ws_stream, response) = WebSocketStream::connect("ws://localhost:5673")
        .await
        .unwrap();

    println!("{:?}", response);

    let mut connection = Connection::builder()
        .container_id("connection-1")
        // .sasl_profile(SaslProfile::Plain { username: String::from("guest"), password: String::from("guest") })
        .open_with_stream(ws_stream)
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();

    let outcome = sender.send("hello AMQP").await.unwrap();
    outcome.accepted_or_else(|outcome| outcome).unwrap();

    sender.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
