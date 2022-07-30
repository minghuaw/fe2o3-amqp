use fe2o3_amqp::{session::Session, Connection, Sender};
use fe2o3_amqp_ws::WebSocketStream;

#[tokio::main]
async fn main() {
    let (ws_stream, _response) = WebSocketStream::connect("ws://localhost:5673")
        .await
        .unwrap();
    let mut connection = Connection::builder()
        .container_id("connection-1")
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
