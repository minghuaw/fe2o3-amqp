use fe2o3_amqp::{
    Connection, Sender, Session, sasl_profile::SaslProfile,
};
use fe2o3_amqp_ws::WebSocketStream;
use dotenv::dotenv;
use tokio::net::TcpStream;
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let hostname = env::var("HOST").unwrap();
    let port = 443;
    let sas_key_name = env::var("SAS_KEY_NAME").unwrap();
    let sas_key_value = env::var("SAS_KEY_VALUE").unwrap();
    let queue_name = env::var("QUEUE_NAME").unwrap();

    // wss://[sas-policy]:[sas-key]@[ns].servicebus.windows.net/$servicebus/websocket
    let ws_address = format!("wss://{sas_key_name}:{sas_key_value}@{hostname}/$servicebus/websocket");

    let stream = TcpStream::connect((&hostname[..], port)).await.unwrap();
    let (ws_stream, _) = WebSocketStream::connect_tls_with_stream(ws_address, stream).await.unwrap();

    let mut connection = Connection::builder()
        .container_id("rust-connection-1")
        .hostname(&hostname[..])
        .sasl_profile(SaslProfile::Plain {
            username: sas_key_name,
            password: sas_key_value,
        })
        .open_with_stream(ws_stream)
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();

    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", queue_name)
        .await
        .unwrap();

    let outcome = sender.send("hello AMQP from rust over websocket").await.unwrap();
    outcome.accepted_or_else(|outcome| outcome).unwrap();
    sender.close().await.unwrap();

    session.end().await.unwrap();
    connection.close().await.unwrap();
}