use dotenv::dotenv;
use fe2o3_amqp::{
    sasl_profile::SaslProfile, types::messaging::Data, Connection, Receiver, Session,
};
use fe2o3_amqp_ws::WebSocketStream;
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let hostname = env::var("HOST_NAME").unwrap();
    let sa_key_name = env::var("SHARED_ACCESS_KEY_NAME").unwrap();
    let sa_key_value = env::var("SHARED_ACCESS_KEY_VALUE").unwrap();
    let queue_name = env::var("QUEUE_NAME").unwrap();

    // wss://[sas-policy]:[sas-key]@[ns].servicebus.windows.net/$servicebus/websocket
    let ws_address = format!("wss://{sa_key_name}:{sa_key_value}@{hostname}/$servicebus/websocket");

    let ws_stream = WebSocketStream::connect(ws_address).await.unwrap();

    let mut connection = Connection::builder()
        .container_id("rust-connection-1")
        .hostname(&hostname[..])
        .sasl_profile(SaslProfile::Plain {
            username: sa_key_name,
            password: sa_key_value,
        })
        .open_with_stream(ws_stream)
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut receiver = Receiver::attach(&mut session, "rust-receiver-link-1", queue_name)
        .await
        .unwrap();

    // All of the Microsoft AMQP clients represent the event body as an uninterpreted bag of bytes.
    let delivery = receiver.recv::<Data>().await.unwrap();
    let msg = std::str::from_utf8(&delivery.body().0).unwrap();
    println!("Received: {:?}", msg);
    receiver.accept(&delivery).await.unwrap();

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
