# fe2o3-amqp-ws

WebSocket adapter for AMQP 1.0 websocket binding

This provides a thin wrapper over `tokio_tungstenite::WebSocketStream`, and the wrapper performs
the WebSocket handshake with the "Sec-WebSocket-Protocol" HTTP header set to "amqp".

Every public constructor of `WebSocketStream` sets the "Sec-WebSocket-Protocol" header to
"amqp" and rejects a handshake response that does not return the same value. To do the
handshake over a stream that you open yourself, use `WebSocketStream::connect_with_stream` or
`WebSocketStream::connect_with_stream_and_config`. This crate has no public constructor that
wraps a `tokio_tungstenite::WebSocketStream` that you built with the raw `tokio_tungstenite`
API.

## Re-exports

This crate re-exports `tungstenite` and `tokio_tungstenite` so that downstream
users can reference the same versions used internally:

```rust
use fe2o3_amqp_ws::tungstenite;
use fe2o3_amqp_ws::tokio_tungstenite;
```

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

## TLS

These three connect functions are not feature gated:
`WebSocketStream::connect_tls_with_config`, `WebSocketStream::connect_tls_with_stream_and_config`,
and `WebSocketStream::connect_tls_with_stream`. The first two take a TLS `connector`. The third
is a shorthand that supplies no connector. A library crate can depend on `fe2o3-amqp-ws` with
`default-features = false`, call these functions, and let the application that builds the final
binary select the TLS stack.

The four features above select which TLS stack `tokio-tungstenite` links. They do not control
whether the connect functions exist. Enable the feature on `fe2o3-amqp-ws` itself. A TLS
feature that another crate enables on `tokio-tungstenite` does not enable the TLS code path of
this crate, because a crate cannot read the features of its dependency.

With none of the four features enabled, these three functions have no TLS stack. A `ws://`
address connects in plaintext. A `wss://` address returns
`Error::Tungstenite(tungstenite::Error::Url(tungstenite::error::UrlError::TlsFeatureNotEnabled))`
before a socket is opened, for every value of `connector`. A `wss://` address is never
downgraded to a plaintext connection.

`WebSocketStream::connect` and `WebSocketStream::connect_with_config` behave differently. They
take no connector and pass `None` to `tokio-tungstenite`, which then selects the TLS stack from
its own enabled features. Cargo unifies features over the whole dependency graph, so another
crate can change that selection. These two functions can therefore make a TLS connection when
`fe2o3-amqp-ws` has no TLS feature enabled. Call `WebSocketStream::connect_tls_with_config` with
`Some(connector)` for a deterministic TLS stack.

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
