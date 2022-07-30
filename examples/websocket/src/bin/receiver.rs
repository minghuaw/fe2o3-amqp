use fe2o3_amqp::{types::primitives::Value, Connection, Delivery, Receiver, Session};
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
    let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
        .await
        .unwrap();

    let delivery: Delivery<Value> = receiver.recv().await.unwrap();
    receiver.accept(&delivery).await.unwrap();

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
