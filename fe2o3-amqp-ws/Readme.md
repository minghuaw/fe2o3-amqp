# fe2o3-amqp-ws

WebSocket adapter for AMQP 1.0 websocket binding

This provides a thin wrapper over `tokio_tungstenite::WebSocketStream`, and the wrapper performs
the WebSocket handshake with the "Sec-WebSocket-Protocol" HTTP header set to "amqp".

The wrapper type [`WebSocketStream`] could also be used for non-AMQP applications; however, the
user should establish websocket stream with raw `tokio_tungstenite` API and then wrap the stream
with the wrapper by `fe2o3_amqp_ws::WebSocketStream::from(ws_stream)`.

## Feature flags

```toml
default = []
```

| Feature | Description |
|---------|-------------|
| `native-tls` | Enables "tokio-tungstenite/native-tls" |
| `native-tls-vendored` | Enables "tokio-tungstenite/native-tls-vendored" |
| `rustls-tls-native-roots` | Enables "tokio-tungstenite/rustls-tls-native-roots" |
| `rustls-tls-webpki-roots` | Enables "tokio-tungstenite/rustls-tls-webpki-roots" |

## Example

```rust
use fe2o3_amqp::{
    types::{messaging::Outcome, primitives::Value},
    Connection, Delivery, Receiver, Sender, Session,
};
use fe2o3_amqp_ws::WebSocketStream;

#[tokio::main]
async fn main() {
    let ws_stream = WebSocketStream::connect("ws://localhost:5673")
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
    let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
        .await
        .unwrap();

    let fut = sender.send_batchable("hello batchable AMQP").await.unwrap();

    let delivery: Delivery<Value> = receiver.recv().await.unwrap();
    receiver.accept(&delivery).await.unwrap();

    let outcome: Outcome = fut.await.unwrap();
    outcome.accepted_or_else(|state| state).unwrap(); // Handle delivery outcome

    sender.close().await.unwrap();
    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
```

### WebAssembly support

Experimental support for `wasm32-unknown-unknown` target has been added since "0.3.0" and uses a
`web_sys::WebSocket` internally. An example of this can be found in
[examples/wasm32-in-browser](https://github.com/minghuaw/fe2o3-amqp/tree/main/examples/wasm32-in-browser).

License: MIT/Apache-2.0
